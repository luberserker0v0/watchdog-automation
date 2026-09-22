//! Deterministic graph transitions for pipeline execution.
//!
//! Every node carries a children list (`n >= 0` outgoing `Next` edges):
//! `n == 0` completes only at an Endpoint; otherwise it fails the run.
//! `n >= 1` tries the children round-robin
//! against the parent's timeout quota. There is no separate single-next
//! path; `n == 1` simply selects the only child every round.
use super::{ChildrenState, ExecutionContext, RunHandle};
use crate::modules::pipeline::authoring::models::node::NodeKind;

/// Whether the children-trying state follows normal edges or an error edge.
/// The error kind uses the parent's `error_timeout` quota instead of
/// `timeout`, while progress keeps showing on the parent node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildrenKind {
    Next,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    Children {
        parent: String,
        candidates: Vec<String>,
        kind: ChildrenKind,
    },
    Complete,
    Fail {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptOutcome {
    Retry,
    Transition(Transition),
    Fail { reason: String },
    Stopped,
}

pub(crate) fn next(ctx: &ExecutionContext) -> Transition {
    let candidates = ctx.plan.next(&ctx.current_node_id).to_vec();
    if candidates.is_empty() {
        if ctx
            .graph
            .find_node(&ctx.current_node_id)
            .is_some_and(|node| node.kind == NodeKind::Endpoint)
        {
            Transition::Complete
        } else {
            Transition::Fail {
                reason: format!(
                    "Pipeline ended at non-Endpoint node '{}'",
                    ctx.current_node_id
                ),
            }
        }
    } else {
        Transition::Children {
            parent: ctx.current_node_id.clone(),
            candidates,
            kind: ChildrenKind::Next,
        }
    }
}

/// Route the failed node's error edge as a single-candidate children state
/// so the error target executes under the failed node's quota
/// (`error_timeout`) with progress still on the failed (parent) node.
pub(crate) fn error(
    ctx: &ExecutionContext,
    failed_node_id: &str,
    reason: String,
) -> AttemptOutcome {
    match ctx.plan.error(failed_node_id) {
        Some(node_id) => AttemptOutcome::Transition(Transition::Children {
            parent: failed_node_id.to_string(),
            candidates: vec![node_id.to_string()],
            kind: ChildrenKind::Error,
        }),
        None => AttemptOutcome::Fail {
            reason: format!("Node '{failed_node_id}': {reason} (no error edge defined)"),
        },
    }
}

fn clear_children_state(handle: &RunHandle) {
    if let Ok(mut parent) = handle.children_parent_id.lock() {
        *parent = None;
    }
    if let Ok(mut started) = handle.children_start_time.lock() {
        *started = None;
    }
    if let Ok(mut child) = handle.children_child_id.lock() {
        *child = None;
    }
    if let Ok(mut timeout) = handle.children_timeout_ms.lock() {
        *timeout = None;
    }
}

pub(crate) fn apply(ctx: &mut ExecutionContext, handle: &RunHandle, transition: Transition) {
    ctx.children = None;
    clear_children_state(handle);
    match transition {
        Transition::Complete => ctx.current_node_id.clear(),
        Transition::Fail { reason } => {
            ctx.failed = true;
            ctx.failure_error = Some(reason);
            ctx.current_node_id.clear();
        }
        Transition::Children {
            parent,
            candidates,
            kind,
        } => {
            let started = std::time::Instant::now();
            let timeout_ms = match kind {
                ChildrenKind::Next => ctx
                    .graph
                    .find_node(&parent)
                    .map(|node| node.timeout)
                    .unwrap_or_default(),
                ChildrenKind::Error => ctx
                    .graph
                    .find_node(&parent)
                    .map(|node| node.error_timeout)
                    .unwrap_or_default(),
            };
            ctx.current_node_id = candidates[0].clone();
            ctx.children = Some(ChildrenState {
                parent_node_id: parent.clone(),
                candidates,
                current_index: 0,
                start_time: started,
                timeout_ms,
                kind,
            });
            if let Ok(mut parent_slot) = handle.children_parent_id.lock() {
                *parent_slot = Some(parent);
            }
            if let Ok(mut started_slot) = handle.children_start_time.lock() {
                *started_slot = Some(started);
            }
            if let Ok(mut timeout_slot) = handle.children_timeout_ms.lock() {
                *timeout_slot = Some(timeout_ms);
            }
        }
    }
}

pub(crate) fn redirect(ctx: &mut ExecutionContext, handle: &RunHandle, target: &str) {
    ctx.children = None;
    clear_children_state(handle);
    ctx.failed = false;
    ctx.failure_error = None;
    // A recovery starts a new execution epoch. Nodes visited before the
    // failure must be eligible to run again instead of being skipped by their
    // previous max_hit counters.
    ctx.hit_counts.clear();
    ctx.current_node_id = target.to_string();
    if let Ok(mut hit_counts) = handle.hit_counts.lock() {
        hit_counts.clear();
    }
    if let Ok(mut current_node) = handle.current_node_id.lock() {
        *current_node = Some(target.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::pipeline::authoring::models::edge::{Edge, EdgeType};
    use crate::modules::pipeline::authoring::models::graph::PipelineGraph;
    use crate::modules::pipeline::authoring::models::node::{Node, NodeConfig, NodeKind};
    use crate::modules::pipeline::plugins::PluginRegistry;
    use crate::modules::pipeline::runtime::plan::ExecutionPlan;
    use std::collections::HashMap;

    #[test]
    fn outcome_variants_are_explicit() {
        assert!(matches!(AttemptOutcome::Retry, AttemptOutcome::Retry));
        assert!(matches!(AttemptOutcome::Stopped, AttemptOutcome::Stopped));
    }

    fn node(id: &str, timeout: u64, error_timeout: u64) -> Node {
        Node {
            id: id.to_string(),
            label: String::new(),
            config: NodeConfig {
                timeout,
                error_timeout,
                ..Default::default()
            },
            params: Vec::new(),
        }
    }

    fn edge(from: &str, to: &str, edge_type: EdgeType) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type,
        }
    }

    fn context(nodes: Vec<Node>, edges: Vec<Edge>, current: &str) -> ExecutionContext {
        let mut nodes = nodes;
        // Entry is required: the first fixture node carries the marker.
        if let Some(first) = nodes.first_mut() {
            first.kind = NodeKind::Entry;
        }
        let graph = PipelineGraph {
            version: "1".to_string(),
            target: None,
            launch: None,
            nodes,
            edges,
        };
        let plan = ExecutionPlan::compile(graph).unwrap();
        let mut ctx = ExecutionContext::new(plan, PluginRegistry::new(), HashMap::new());
        ctx.current_node_id = current.to_string();
        ctx
    }

    #[test]
    fn single_child_is_children_not_next() {
        let ctx = context(
            vec![node("A", 100, 50), node("B", 100, 50)],
            vec![edge("A", "B", EdgeType::Next)],
            "A",
        );
        assert!(matches!(
            next(&ctx),
            Transition::Children { parent, candidates, kind: ChildrenKind::Next }
                if parent == "A" && candidates == vec!["B".to_string()]
        ));
    }

    #[test]
    fn multiple_children_is_children() {
        let ctx = context(
            vec![node("A", 100, 50), node("B", 100, 50), node("C", 100, 50)],
            vec![
                edge("A", "B", EdgeType::Next),
                edge("A", "C", EdgeType::Next),
            ],
            "A",
        );
        assert!(matches!(
            next(&ctx),
            Transition::Children { candidates, kind: ChildrenKind::Next, .. }
                if candidates.len() == 2
        ));
    }

    #[test]
    fn only_endpoint_without_children_completes() {
        let mut ctx = context(vec![node("A", 100, 50), node("Done", 100, 50)], vec![], "A");
        assert!(matches!(next(&ctx), Transition::Fail { .. }));
        let handle = RunHandle::new("test".to_string());
        let failure = next(&ctx);
        apply(&mut ctx, &handle, failure);
        assert!(ctx.failed);
        assert!(ctx
            .failure_error
            .as_deref()
            .unwrap()
            .contains("non-Endpoint node 'A'"));

        ctx.graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "Done")
            .unwrap()
            .kind = NodeKind::Endpoint;
        ctx.current_node_id = "Done".to_string();
        assert!(matches!(next(&ctx), Transition::Complete));
    }

    #[test]
    fn error_routes_with_error_kind_and_budget() {
        let ctx = context(
            vec![node("A", 100, 50), node("E", 100, 50)],
            vec![edge("A", "E", EdgeType::Error)],
            "B",
        );
        let handle = RunHandle::new("test".to_string());
        let mut ctx = ctx;
        match error(&ctx, "A", "boom".to_string()) {
            AttemptOutcome::Transition(transition) => {
                apply(&mut ctx, &handle, transition);
            }
            AttemptOutcome::Fail { .. } => panic!("expected error transition"),
            AttemptOutcome::Retry | AttemptOutcome::Stopped => {
                panic!("unexpected outcome")
            }
        }
        let children = ctx.children.as_ref().expect("children state");
        assert_eq!(children.parent_node_id, "A");
        assert_eq!(children.candidates, vec!["E".to_string()]);
        assert_eq!(children.kind, ChildrenKind::Error);
        assert_eq!(children.timeout_ms, 50);
        assert_eq!(ctx.current_node_id, "E");
        assert_eq!(
            handle.children_parent_id.lock().unwrap().as_deref(),
            Some("A")
        );
        assert_eq!(*handle.children_timeout_ms.lock().unwrap(), Some(50));
    }

    #[test]
    fn apply_next_children_uses_parent_timeout() {
        let mut ctx = context(
            vec![node("A", 100, 50), node("B", 100, 50)],
            vec![edge("A", "B", EdgeType::Next)],
            "A",
        );
        let handle = RunHandle::new("test".to_string());
        let transition = next(&ctx);
        apply(&mut ctx, &handle, transition);
        let children = ctx.children.as_ref().expect("children state");
        assert_eq!(children.timeout_ms, 100);
        assert_eq!(ctx.current_node_id, "B");
    }

    #[test]
    fn recovery_redirect_clears_failure_children_and_hit_counts() {
        let mut ctx = context(
            vec![node("A", 100, 50), node("B", 100, 50)],
            vec![edge("A", "B", EdgeType::Next)],
            "A",
        );
        let handle = RunHandle::new("run".into());
        ctx.failed = true;
        ctx.failure_error = Some("boom".into());
        ctx.hit_counts.insert("B".into(), 1);
        redirect(&mut ctx, &handle, "B");
        assert_eq!(ctx.current_node_id, "B");
        assert!(!ctx.failed);
        assert!(ctx.failure_error.is_none());
        assert!(ctx.hit_counts.is_empty());
        assert!(ctx.children.is_none());
        assert!(handle.hit_counts.lock().unwrap().is_empty());
        assert_eq!(handle.current_node_id.lock().unwrap().as_deref(), Some("B"));
    }

    #[test]
    fn error_without_edge_fails() {
        let ctx = context(vec![node("A", 100, 50)], vec![], "A");
        assert!(matches!(
            error(&ctx, "A", "boom".to_string()),
            AttemptOutcome::Fail { .. }
        ));
    }
}

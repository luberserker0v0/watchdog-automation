//! Immutable, validated execution representation for a pipeline run.
//!
//! Parsing Mermaid and metadata is intentionally kept at the boundary.  The
//! engine receives this plan so routing never needs to scan the whole edge list
//! while a capture session is active.
use crate::modules::pipeline::models::edge::EdgeType;
use crate::modules::pipeline::models::graph::PipelineGraph;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct NodeRoute {
    pub next: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    graph: PipelineGraph,
    entry_node_id: String,
    routes: HashMap<String, NodeRoute>,
}

impl ExecutionPlan {
    pub fn compile(graph: PipelineGraph) -> Result<Self, Vec<String>> {
        let mut errors = Vec::new();
        let mut routes: HashMap<String, NodeRoute> = HashMap::new();
        let mut known = HashSet::new();

        for node in &graph.nodes {
            if !known.insert(node.id.as_str()) {
                errors.push(format!("Duplicate node ID: {}", node.id));
            }
            routes.entry(node.id.clone()).or_default();
        }

        for edge in &graph.edges {
            if !known.contains(edge.from.as_str()) || !known.contains(edge.to.as_str()) {
                errors.push(format!("Edge '{} -> {}' references an unknown node", edge.from, edge.to));
                continue;
            }
            let route = routes.entry(edge.from.clone()).or_default();
            match edge.edge_type {
                EdgeType::Next => route.next.push(edge.to.clone()),
                EdgeType::Error => {
                    if route.error.replace(edge.to.clone()).is_some() {
                        errors.push(format!("Node '{}' has more than one error edge", edge.from));
                    }
                }
            }
        }

        let entry_node_id = match entry_nodes(&graph) {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(error);
                String::new()
            }
        };
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Self { graph, entry_node_id, routes })
    }

    pub fn graph(&self) -> &PipelineGraph { &self.graph }
    pub fn into_graph(self) -> PipelineGraph { self.graph }
    pub fn entry_node_id(&self) -> &str { &self.entry_node_id }
    pub fn route(&self, node_id: &str) -> Option<&NodeRoute> { self.routes.get(node_id) }
    pub fn next(&self, node_id: &str) -> &[String] {
        self.route(node_id).map(|route| route.next.as_slice()).unwrap_or(&[])
    }
    pub fn error(&self, node_id: &str) -> Option<&str> {
        self.route(node_id).and_then(|route| route.error.as_deref())
    }
}

/// Resolve the run entry node. Exactly one `Entry`-kind node is required;
/// the marker is authoritative wherever it sits.
fn entry_nodes(graph: &PipelineGraph) -> Result<String, String> {
    use crate::modules::pipeline::models::node::NodeKind;
    let explicit: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Entry)
        .map(|node| node.id.as_str())
        .collect();
    match explicit.len() {
        1 => Ok(explicit[0].to_string()),
        0 => Err(
            "Pipeline must declare exactly one Entry node (node kind \"Entry\"); found none"
                .to_string(),
        ),
        n => Err(format!(
            "Pipeline must have exactly one Entry node; found {n}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::pipeline::models::{edge::Edge, graph::PipelineGraph, node::Node};

    #[test]
    fn prefers_explicit_entry_node() {
        use crate::modules::pipeline::models::node::{NodeConfig, NodeKind};
        let entry_node = |id: &str, kind: NodeKind| Node {
            id: id.into(),
            label: String::new(),
            config: NodeConfig {
                kind,
                ..Default::default()
            },
            params: vec![],
        };
        let graph = PipelineGraph { version: "1".into(), desktop: None, target: None, launch: None,
            nodes: vec![entry_node("Start", NodeKind::Entry), entry_node("Work", NodeKind::Normal)],
            edges: vec![Edge { from: "Start".into(), to: "Work".into(), edge_type: EdgeType::Next }] };
        let plan = ExecutionPlan::compile(graph).unwrap();
        assert_eq!(plan.entry_node_id(), "Start");
    }

    #[test]
    fn preserves_mermaid_next_order_for_children() {
        use crate::modules::pipeline::models::node::{NodeConfig, NodeKind};
        let plain = |id: &str| Node { id: id.into(), label: String::new(), config: Default::default(), params: vec![] };
        let mut entry = plain("A");
        entry.config = NodeConfig {
            kind: NodeKind::Entry,
            ..Default::default()
        };
        let graph = PipelineGraph { version: "1".into(), desktop: None, target: None, launch: None,
            nodes: vec![entry, plain("B"), plain("C")],
            edges: vec![Edge { from: "A".into(), to: "B".into(), edge_type: EdgeType::Next }, Edge { from: "A".into(), to: "C".into(), edge_type: EdgeType::Next }]
        };
        let plan = ExecutionPlan::compile(graph).unwrap();
        assert_eq!(plan.entry_node_id(), "A");
        assert_eq!(plan.next("A"), ["B", "C"]);
    }

    #[test]
    fn rejects_duplicate_entry_nodes() {
        use crate::modules::pipeline::models::node::{NodeConfig, NodeKind};
        let entry_node = |id: &str| Node {
            id: id.into(),
            label: String::new(),
            config: NodeConfig {
                kind: NodeKind::Entry,
                ..Default::default()
            },
            params: vec![],
        };
        let graph = PipelineGraph { version: "1".into(), desktop: None, target: None, launch: None,
            nodes: vec![entry_node("A"), entry_node("B")],
            edges: vec![Edge { from: "A".into(), to: "B".into(), edge_type: EdgeType::Next }] };
        let errors = ExecutionPlan::compile(graph).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("one Entry node")));
    }
}

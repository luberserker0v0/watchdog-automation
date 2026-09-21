//! Execution-result recording and transition policies.
//!
//! This module is intentionally independent from recognition and capture: a
//! completed/skipped step must update the runtime snapshot and event stream in
//! exactly one place.
use super::{ExecutionContext, RunHandle, StepResult};
use tauri::{AppHandle, Emitter};

fn publish_status(app: &AppHandle, handle: &RunHandle, appended_step: Option<StepResult>) {
    let _ = app.emit("pipeline-status", handle.event(appended_step));
}

pub(crate) fn record(
    ctx: &mut ExecutionContext,
    handle: &RunHandle,
    app: &AppHandle,
    steps: &mut Vec<StepResult>,
    step: StepResult,
) {
    ctx.step_results.push(step.clone());
    if let Ok(mut results) = handle.step_results.lock() {
        results.push(step.clone());
    }
    steps.push(step.clone());
    publish_status(app, handle, Some(step));
}

/// Returns true when the transition has reached a terminal state.
pub(crate) fn skip(
    ctx: &mut ExecutionContext,
    handle: &RunHandle,
    app: &AppHandle,
    steps: &mut Vec<StepResult>,
    node_id: &str,
    record_outside_children: bool,
) -> bool {
    let should_record = ctx.children.is_some() || record_outside_children;
    if should_record {
        if let Ok(mut count) = handle.step_count.lock() {
            *count += 1;
        }
        record(
            ctx,
            handle,
            app,
            steps,
            StepResult {
                node_id: node_id.to_string(),
                matched: true,
                confidence: 0.0,
                box_region: None,
                actions_fired: 0,
                skipped: true,
                duration_ms: 0,
                error: None,
                timings_ms: None,
            },
        );
    }
    if let Some(children) = ctx.children.as_mut() {
        children.current_index = (children.current_index + 1) % children.candidates.len();
        ctx.current_node_id = children.candidates[children.current_index].clone();
    } else {
        let transition = if ctx.graph.find_node(node_id).is_some_and(|node| {
            node.kind == crate::modules::pipeline::authoring::models::node::NodeKind::Endpoint
        }) {
            super::routing::Transition::Fail {
                reason: format!("Endpoint node '{node_id}' was skipped"),
            }
        } else {
            super::routing::next(ctx)
        };
        super::routing::apply(ctx, handle, transition);
    }
    ctx.current_node_id.is_empty()
}

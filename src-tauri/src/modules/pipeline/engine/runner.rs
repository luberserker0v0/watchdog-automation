use super::{EngineResult, ExecutionContext, PipelineState, RunHandle, StepResult};
use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::edge::EdgeType;
use crate::modules::pipeline::models::graph::PipelineGraph;
use crate::modules::pipeline::plugins::PluginRegistry;
use crate::services::stream::StreamService;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub async fn run_pipeline(
    graph: PipelineGraph,
    registry: PluginRegistry,
    handle: Arc<RunHandle>,
    stream_service: Arc<StreamService>,
    user_params: HashMap<String, serde_json::Value>,
    save_debug_images: bool,
    detailed_timings: bool,
    pipeline_startup_ms: Option<u64>,
) -> EngineResult {
    let mut ctx = ExecutionContext::new(graph, registry, user_params);
    ctx.save_debug_images = save_debug_images;

    // Resolve hwnd for window targets (used for coordinate conversion)
    if let Some(crate::modules::pipeline::models::graph::PipelineTarget::Window { .. }) =
        &ctx.graph.target
    {
        if let Ok(hwnd) =
            crate::repositories::window::find_window_by_target(&ctx.graph.target.as_ref().unwrap())
        {
            ctx.hwnd = Some(hwnd);
            log::info!(
                "[pipeline] resolved window target hwnd={} for coordinate conversion",
                hwnd
            );
        }
    }

    let mut rx = stream_service.subscribe();

    {
        if let Ok(mut state) = handle.state.lock() {
            *state = PipelineState::Running;
        }
    }

    log::info!("[pipeline] started, entry node='{}'", ctx.current_node_id);

    let mut steps = Vec::new();

    loop {
        if let Ok(stop) = handle.should_stop.lock() {
            if *stop {
                break;
            }
        }

        let is_paused = {
            if let Ok(state) = handle.state.lock() {
                *state == PipelineState::Paused
            } else {
                false
            }
        };
        if is_paused {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        let current_node = match ctx.graph.find_node(&ctx.current_node_id) {
            Some(node) => node.clone(),
            None => {
                break;
            }
        };

        if let Ok(mut node_id) = handle.current_node_id.lock() {
            *node_id = Some(current_node.id.clone());
        }

        if !current_node.enabled {
            follow_edge(&mut ctx, EdgeType::Next);
            if ctx.current_node_id.is_empty() {
                break;
            }
            continue;
        }

        let hit_count = ctx.hit_counts.get(&current_node.id).copied().unwrap_or(0);
        if hit_count >= current_node.max_hit {
            follow_edge(&mut ctx, EdgeType::Next);
            if ctx.current_node_id.is_empty() {
                break;
            }
            continue;
        }

        let step_start = std::time::Instant::now();
        let mut timings = std::collections::BTreeMap::new();
        if let Some(startup_ms) = pipeline_startup_ms.filter(|_| steps.is_empty()) {
            timings.insert("pipeline_startup".to_string(), startup_ms);
        }

        if let Ok(mut count) = handle.step_count.lock() {
            *count += 1;
        }

        log::info!(
            "[pipeline] step {}: node='{}'",
            handle.step_count.lock().map(|c| *c).unwrap_or(0),
            current_node.id
        );

        let phase_start = std::time::Instant::now();
        let captured_frame = if stream_service.is_streaming() {
            match tokio::time::timeout(Duration::from_secs(3), rx.recv()).await {
                Ok(Ok(frame)) => image::load_from_memory(&frame.bytes)
                    .ok()
                    .map(|image| (image, frame.bounds)),
                Ok(Err(e)) => {
                    log::warn!(
                        "[pipeline] node='{}' stream recv error: {}",
                        current_node.id,
                        e
                    );
                    None
                }
                Err(_) => {
                    log::warn!(
                        "[pipeline] node='{}' stream frame timeout (3s)",
                        current_node.id
                    );
                    None
                }
            }
        } else {
            log::warn!("[pipeline] node='{}' stream not running", current_node.id);
            None
        };
        if detailed_timings {
            timings.insert(
                "frame_acquire".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }

        let (screenshot, frame_bounds) = match captured_frame {
            Some(frame) => frame,
            None => {
                log::warn!(
                    "[pipeline] node='{}' screenshot capture failed",
                    current_node.id
                );
                let step = StepResult {
                    node_id: current_node.id.clone(),
                    matched: false,
                    confidence: 0.0,
                    box_region: None,
                    actions_fired: 0,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    error: Some("Failed to capture screen".to_string()),
                    timings_ms: detailed_timings.then_some(timings),
                };
                steps.push(step.clone());
                ctx.step_results.push(step);
                if let Ok(mut sr) = handle.step_results.lock() {
                    sr.push(steps.last().unwrap().clone());
                }

                handle_error(&mut ctx);
                if ctx.current_node_id.is_empty() {
                    break;
                }
                continue;
            }
        };

        let phase_start = std::time::Instant::now();
        let effective_roi =
            crate::modules::pipeline::plugins::shared::resolve_recognition_roi_with_offset(
                current_node.recognition.roi.as_ref(),
                current_node.recognition.roi_offset.as_ref(),
                screenshot.width(),
                screenshot.height(),
            );
        let match_result = match effective_roi.as_ref() {
            Ok(roi) => ctx.registry.evaluate_recognition_with_selection(
                &screenshot,
                &current_node.recognition.kind,
                Some(roi),
                current_node.recognition.candidate_selection.as_ref(),
            ),
            Err(error) => Err(error.clone()),
        };

        let mut match_result = match match_result {
            Ok(r) => r,
            Err(e) => {
                log::warn!(
                    "[pipeline] node='{}' recognition error: {}",
                    current_node.id,
                    e
                );
                let step = StepResult {
                    node_id: current_node.id.clone(),
                    matched: false,
                    confidence: 0.0,
                    box_region: None,
                    actions_fired: 0,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    error: Some(format!("Recognition error: {}", e)),
                    timings_ms: detailed_timings.then_some({
                        timings.insert(
                            "recognition".to_string(),
                            phase_start.elapsed().as_millis() as u64,
                        );
                        timings
                    }),
                };
                steps.push(step.clone());
                ctx.step_results.push(step);
                if let Ok(mut sr) = handle.step_results.lock() {
                    sr.push(steps.last().unwrap().clone());
                }

                handle_error(&mut ctx);
                if ctx.current_node_id.is_empty() {
                    break;
                }
                continue;
            }
        };
        if detailed_timings {
            timings.insert(
                "recognition".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }
        let debug_match_result = match_result.clone();

        // Convert capture-relative recognition pixels to physical virtual-screen pixels.
        let phase_start = std::time::Instant::now();
        if let Some(region) = match_result.box_region.as_mut() {
            let local = region.clone();
            let converted = if let Some(bounds) = frame_bounds {
                Ok(crate::modules::pipeline::plugins::shared::map_region(
                    &local,
                    screenshot.width(),
                    screenshot.height(),
                    bounds,
                ))
            } else {
                match &ctx.graph.target {
                    Some(crate::modules::pipeline::models::graph::PipelineTarget::Window {
                        capture_area,
                        ..
                    }) => ctx
                        .hwnd
                        .ok_or_else(|| "Window target has no resolved HWND".to_string())
                        .and_then(|hwnd| {
                            crate::modules::pipeline::plugins::shared::window_region_to_screen(
                                hwnd,
                                *capture_area,
                                &local,
                                screenshot.width(),
                                screenshot.height(),
                            )
                        }),
                    Some(crate::modules::pipeline::models::graph::PipelineTarget::Screen {
                        index,
                    }) => crate::modules::pipeline::plugins::shared::screen_region_to_screen(
                        *index,
                        &local,
                        screenshot.width(),
                        screenshot.height(),
                    ),
                    None => Ok(local.clone()),
                }
            };
            match converted {
                Ok(screen_region) => {
                    log::info!(
                        "[pipeline] node='{}' region conversion: local=({}, {}, {}x{}), frame={}x{}, screen=({}, {}, {}x{}), click=({},{})",
                        current_node.id, local.x, local.y, local.width, local.height,
                        screenshot.width(), screenshot.height(), screen_region.x, screen_region.y,
                        screen_region.width, screen_region.height,
                        screen_region.center().0, screen_region.center().1,
                    );
                    *region = screen_region;
                }
                Err(error) => log::warn!(
                    "[pipeline] node='{}' coordinate conversion failed: {}",
                    current_node.id,
                    error
                ),
            }
        }
        if detailed_timings {
            timings.insert(
                "coordinate".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }

        let matched = crate::modules::pipeline::models::recognizer::effective_match(
            &match_result,
            current_node.inverse,
        );

        if !matched {
            log::info!(
                "[pipeline] node='{}' not matched, confidence={:.3}",
                current_node.id,
                match_result.confidence
            );

            if ctx.save_debug_images {
                let phase_start = std::time::Instant::now();
                log::info!(
                    "[pipeline] node='{}' saving debug image (matched=false)",
                    current_node.id
                );
                if let Err(e) = super::debug_image::save_debug_image(
                    &screenshot,
                    &current_node.id,
                    handle.step_count.lock().map(|c| *c).unwrap_or(0),
                    &current_node.recognition.kind,
                    effective_roi.as_ref().ok(),
                    &debug_match_result,
                    &[],
                    &std::collections::HashMap::new(),
                ) {
                    log::warn!(
                        "[pipeline] node='{}' failed to save debug image: {}",
                        current_node.id,
                        e
                    );
                }
                if detailed_timings {
                    timings.insert(
                        "debug_image".to_string(),
                        phase_start.elapsed().as_millis() as u64,
                    );
                }
            }

            let step = StepResult {
                node_id: current_node.id.clone(),
                matched: false,
                confidence: match_result.confidence,
                box_region: None,
                actions_fired: 0,
                duration_ms: step_start.elapsed().as_millis() as u64,
                error: match_result.selection_error.clone(),
                timings_ms: detailed_timings.then_some(timings),
            };
            steps.push(step.clone());
            ctx.step_results.push(step);
            if let Ok(mut sr) = handle.step_results.lock() {
                sr.push(steps.last().unwrap().clone());
            }

            handle_error(&mut ctx);
            if ctx.current_node_id.is_empty() {
                break;
            }
            continue;
        }

        if let Some(region) = &match_result.box_region {
            ctx.recognized_regions
                .insert(current_node.id.clone(), region.clone());
        }
        if let Some(region) = &debug_match_result.box_region {
            ctx.debug_recognized_regions
                .insert(current_node.id.clone(), region.clone());
        }
        *ctx.hit_counts.entry(current_node.id.clone()).or_insert(0) += 1;

        log::info!(
            "[pipeline] node='{}' matched, confidence={:.3}{}",
            current_node.id,
            match_result.confidence,
            match &match_result.text {
                Some(t) => format!(", text=\"{}\"", t),
                None => String::new(),
            }
        );

        let phase_start = std::time::Instant::now();
        tokio::time::sleep(Duration::from_millis(current_node.pre_delay)).await;
        if detailed_timings {
            timings.insert(
                "pre_delay".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }

        let substituted_actions = substitute_actions(&current_node.actions, &ctx.user_params);

        let mut actions_fired = 0;
        let mut action_error = None;

        let phase_start = std::time::Instant::now();
        for action in &substituted_actions {
            match ctx
                .registry
                .execute_action(
                    action,
                    match_result.box_region.as_ref(),
                    &ctx.recognized_regions,
                )
                .await
            {
                Ok(()) => {
                    actions_fired += 1;
                }
                Err(e) => {
                    action_error = Some(format!("Action error: {}", e));
                    break;
                }
            }

            for _ in 1..current_node.repeat {
                tokio::time::sleep(Duration::from_millis(current_node.repeat_delay)).await;
                match ctx
                    .registry
                    .execute_action(
                        action,
                        match_result.box_region.as_ref(),
                        &ctx.recognized_regions,
                    )
                    .await
                {
                    Ok(()) => {
                        actions_fired += 1;
                    }
                    Err(e) => {
                        action_error = Some(format!("Action repeat error: {}", e));
                        break;
                    }
                }
            }

            if action_error.is_some() {
                break;
            }
        }
        if detailed_timings {
            timings.insert(
                "actions".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }

        let phase_start = std::time::Instant::now();
        tokio::time::sleep(Duration::from_millis(current_node.post_delay)).await;
        if detailed_timings {
            timings.insert(
                "post_delay".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }

        log::info!(
            "[pipeline] node='{}' actions fired: {}",
            current_node.id,
            actions_fired
        );

        // Save debug image: one per node step
        if ctx.save_debug_images {
            let phase_start = std::time::Instant::now();
            log::info!(
                "[pipeline] node='{}' saving debug image (matched=true, actions={})",
                current_node.id,
                actions_fired
            );
            if let Err(e) = super::debug_image::save_debug_image(
                &screenshot,
                &current_node.id,
                handle.step_count.lock().map(|c| *c).unwrap_or(0),
                &current_node.recognition.kind,
                effective_roi.as_ref().ok(),
                &debug_match_result,
                &substituted_actions,
                &ctx.debug_recognized_regions,
            ) {
                log::warn!(
                    "[pipeline] node='{}' failed to save debug image: {}",
                    current_node.id,
                    e
                );
            }
            if detailed_timings {
                timings.insert(
                    "debug_image".to_string(),
                    phase_start.elapsed().as_millis() as u64,
                );
            }
        }

        let step = StepResult {
            node_id: current_node.id.clone(),
            matched: true,
            confidence: match_result.confidence,
            box_region: match_result.box_region,
            actions_fired,
            duration_ms: step_start.elapsed().as_millis() as u64,
            error: action_error.clone(),
            timings_ms: detailed_timings.then_some(timings),
        };
        steps.push(step.clone());
        ctx.step_results.push(step);
        if let Ok(mut sr) = handle.step_results.lock() {
            sr.push(steps.last().unwrap().clone());
        }

        if action_error.is_some() {
            handle_error(&mut ctx);
            if ctx.current_node_id.is_empty() {
                break;
            }
            continue;
        }

        follow_edge(&mut ctx, EdgeType::Next);
        if ctx.current_node_id.is_empty() {
            break;
        }
    }

    let final_state = if ctx.failed {
        let failed_node = if ctx.current_node_id.is_empty() {
            "unknown".to_string()
        } else {
            ctx.current_node_id.clone()
        };
        PipelineState::Failed(format!("No error edge defined for node '{}'", failed_node))
    } else if ctx.current_node_id.is_empty() {
        PipelineState::Completed
    } else {
        PipelineState::Failed("Pipeline stopped".to_string())
    };

    {
        if let Ok(mut state) = handle.state.lock() {
            *state = final_state.clone();
        }
    }

    log::info!(
        "[pipeline] finished: state={:?}, steps={}",
        final_state,
        steps.len()
    );

    EngineResult {
        success: final_state == PipelineState::Completed,
        steps,
        error: match final_state {
            PipelineState::Failed(msg) => Some(msg),
            _ => None,
        },
    }
}

fn follow_edge(ctx: &mut ExecutionContext, edge_type: EdgeType) {
    let edges = match edge_type {
        EdgeType::Next => ctx.graph.next_edges(&ctx.current_node_id),
        EdgeType::Error => ctx.graph.error_edges(&ctx.current_node_id),
    };

    if let Some(edge) = edges.first() {
        ctx.current_node_id = edge.to.clone();
    } else {
        ctx.current_node_id = String::new();
    }
}

fn handle_error(ctx: &mut ExecutionContext) {
    let error_edges = ctx.graph.error_edges(&ctx.current_node_id);

    if let Some(edge) = error_edges.first() {
        ctx.current_node_id = edge.to.clone();
    } else {
        log::warn!(
            "[pipeline] node '{}' has no error edge — failing pipeline",
            ctx.current_node_id
        );
        ctx.failed = true;
        ctx.current_node_id = String::new();
    }
}

pub(crate) fn substitute_actions(
    actions: &[Action],
    params: &HashMap<String, serde_json::Value>,
) -> Vec<Action> {
    if params.is_empty() {
        return actions.to_vec();
    }
    actions
        .iter()
        .map(|action| substitute_action(action, params))
        .collect()
}

fn substitute_action(action: &Action, params: &HashMap<String, serde_json::Value>) -> Action {
    match action {
        Action::InputText { input_text } => Action::InputText {
            input_text: substitute_string(input_text, params),
        },
        Action::Custom { action, param } => {
            let new_param = substitute_json(param, params);
            Action::Custom {
                action: action.clone(),
                param: new_param,
            }
        }
        _ => action.clone(),
    }
}

fn substitute_string(s: &str, params: &HashMap<String, serde_json::Value>) -> String {
    let mut result = s.to_string();
    for (key, value) in params {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            serde_json::Value::String(v) => v.clone(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Null => "null".to_string(),
            _ => value.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

fn substitute_json(
    value: &serde_json::Value,
    params: &HashMap<String, serde_json::Value>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => serde_json::Value::String(substitute_string(s, params)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| substitute_json(v, params)).collect())
        }
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), substitute_json(v, params)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod parameter_tests {
    use super::*;

    #[test]
    fn substitutes_tester_values_in_input_actions() {
        let actions = vec![Action::InputText {
            input_text: "{{account}}/{{retries}}/{{enabled}}".to_string(),
        }];
        let params = HashMap::from([
            ("account".to_string(), serde_json::json!("admin")),
            ("retries".to_string(), serde_json::json!(3)),
            ("enabled".to_string(), serde_json::json!(true)),
        ]);

        let substituted = substitute_actions(&actions, &params);
        assert!(matches!(
            &substituted[0],
            Action::InputText { input_text }
                if input_text == "admin/3/true"
        ));
    }
}

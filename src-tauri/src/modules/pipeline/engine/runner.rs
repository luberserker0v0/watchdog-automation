use super::{ChildrenKind, EngineResult, ExecutionContext, PipelineState, RunHandle, StepResult};
use super::frame;
use super::routing;
use super::step;
use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::node::Node;
use crate::modules::pipeline::plan::ExecutionPlan;
use crate::modules::pipeline::plugins::PluginRegistry;
use crate::services::stream::{CaptureEngine, StreamFrame, StreamService, StreamTarget};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

#[derive(Clone)]
struct ActiveCapture {
    configured_key: String,
    stream_target: StreamTarget,
    session_id: u64,
}

fn publish_status(app: &AppHandle, handle: &RunHandle, appended_step: Option<StepResult>) {
    // Delivery is best-effort: `get_pipeline_status` remains the recovery path
    // when a window was not listening while an event was emitted.
    let _ = app.emit("pipeline-status", handle.event(appended_step));
}


async fn ensure_effective_target(
    graph_target: Option<&crate::modules::pipeline::models::graph::PipelineTarget>,
    node_target: Option<&crate::modules::pipeline::models::graph::PipelineTarget>,
    stream_service: &StreamService,
    app: &AppHandle,
    handle: &RunHandle,
    active_capture: &mut Option<ActiveCapture>,
) -> Result<(StreamTarget, u64), String> {
    use crate::modules::pipeline::models::graph::PipelineTarget;
    let configured = node_target.or(graph_target);
    let configured_key = format!("{configured:?}");
    if let Some(active) = active_capture.as_ref()
        .filter(|active| active.configured_key == configured_key && stream_service.is_streaming())
    {
        // The node still uses the same declared target and live capture
        // session. Avoid repeated window enumeration/foreground calls on each
        // recognition retry; action execution performs its own foreground
        // safety check immediately before input is sent.
        return Ok((active.stream_target.clone(), active.session_id));
    }
    let inherited_window_size = node_target
        .and_then(PipelineTarget::window_size)
        .or_else(|| graph_target.and_then(PipelineTarget::window_size));
    let status = stream_service.status().await;
    let (stream_target, effective_target) = match configured {
        Some(PipelineTarget::Screen { index }) => (
            StreamTarget::Screen { index: *index },
            PipelineTarget::Screen { index: *index },
        ),
        Some(
            target @ PipelineTarget::Window {
                process,
                class,
                title,
                capture_area,
                ..
            },
        ) => {
            let hwnd = if target.has_window_filters() {
                crate::repositories::window::find_window_by_target(target)
                    .map_err(|error| format!("Failed to resolve node window target: {error}"))?
            } else if status.is_streaming && status.target_type == "window" {
                status.target_index
            } else {
                return Err(
                    "Filterless node window target requires an active/default window target"
                        .to_string(),
                );
            };
            (
                StreamTarget::Window {
                    hwnd,
                    capture_area: *capture_area,
                },
                PipelineTarget::Window {
                    process: process.clone(),
                    class: class.clone(),
                    title: title.clone(),
                    capture_area: *capture_area,
                    window_size: inherited_window_size,
                },
            )
        }
        None if status.is_streaming && status.target_type == "screen" => (
            StreamTarget::Screen {
                index: status.target_index,
            },
            PipelineTarget::Screen {
                index: status.target_index,
            },
        ),
        None if status.is_streaming && status.target_type == "window" => {
            let capture_area = if status.capture_area == "client" {
                crate::modules::pipeline::models::graph::CaptureArea::Client
            } else {
                crate::modules::pipeline::models::graph::CaptureArea::Frame
            };
            (
                StreamTarget::Window {
                    hwnd: status.target_index,
                    capture_area,
                },
                PipelineTarget::Window {
                    process: None,
                    class: None,
                    title: None,
                    capture_area,
                    window_size: inherited_window_size,
                },
            )
        }
        None => return Err("Pipeline has no effective capture target".to_string()),
    };

    let foreground_changed = match &stream_target {
        StreamTarget::Window { hwnd, .. } => {
            let hwnd = *hwnd;
            tokio::task::spawn_blocking(move || {
                crate::repositories::window::ensure_window_foreground(hwnd)
            })
            .await
            .map_err(|error| format!("Target foreground worker failed: {error}"))??
        }
        StreamTarget::Screen { .. } => false,
    };

    let matches = status.is_streaming
        && match &stream_target {
            StreamTarget::Screen { index } => {
                status.target_type == "screen" && status.target_index == *index
            }
            StreamTarget::Window { hwnd, capture_area } => {
                status.target_type == "window"
                    && status.target_index == *hwnd
                    && status.capture_area
                        == if *capture_area
                            == crate::modules::pipeline::models::graph::CaptureArea::Client
                        {
                            "client"
                        } else {
                            "frame"
                        }
            }
        }
        && status.engine == "gdi"
        && !foreground_changed;
    if !matches {
        stream_service.stop().await;
        stream_service
            .start(app.clone(), 5, stream_target.clone(), CaptureEngine::Gdi)
            .await?;
        let (kind, hwnd) = match &stream_target {
            StreamTarget::Screen { .. } => ("screen", None),
            StreamTarget::Window { hwnd, .. } => ("window", Some(*hwnd)),
        };
        let _ = app.emit(
            "update_capture",
            serde_json::json!({"type":kind,"action":"sync","target":effective_target,"hwnd":hwnd}),
        );
    }
    if let Ok(mut target) = handle.effective_target.lock() {
        *target = Some(effective_target);
    }
    let session_id = stream_service.status().await.session_id;
    *active_capture = Some(ActiveCapture {
        configured_key,
        stream_target: stream_target.clone(),
        session_id,
    });
    Ok((stream_target, session_id))
}

fn stop_requested(handle: &RunHandle) -> bool {
    handle
        .should_stop
        .lock()
        .map(|value| *value)
        .unwrap_or(true)
}

/// Single-level timeout: while a children state is active the parent's quota
/// (`timeout` for normal children, `error_timeout` for the error target)
/// bounds every attempt. Only parent-less execution (the entry node) falls
/// back to the node's own timeout.
fn quota_exceeded(
    ctx: &ExecutionContext,
    current_node: &Node,
    solo_start: &Instant,
) -> bool {
    if let Some(children) = ctx.children.as_ref() {
        children.start_time.elapsed().as_millis() as u64 >= children.timeout_ms
    } else {
        solo_start.elapsed().as_millis() as u64 >= current_node.timeout
    }
}

/// Active quota in milliseconds, for timeout diagnostics and progress.
fn quota_ms(ctx: &ExecutionContext, current_node: &Node) -> u64 {
    ctx.children
        .as_ref()
        .map(|children| children.timeout_ms)
        .unwrap_or(current_node.timeout)
}

/// Owner of the active quota: the parent while normal children are tried,
/// otherwise the current node itself (entry execution, or an error target
/// whose own error edge chains next). Quota expiry routes this node's
/// error edge.
fn quota_owner(ctx: &ExecutionContext, current_node: &Node) -> String {
    match ctx.children.as_ref() {
        Some(children) if children.kind == ChildrenKind::Next => {
            children.parent_node_id.clone()
        }
        _ => current_node.id.clone(),
    }
}

fn debug_image_context(
    target: &StreamTarget,
    attempt: usize,
    frame_sequence: u64,
    actions_run_afterward: bool,
) -> super::debug_image::DebugImageContext {
    let (capture_target, capture_area) = match target {
        StreamTarget::Screen { index } => (format!("Screen {}", index), "screen".to_string()),
        StreamTarget::Window { hwnd, capture_area } => (
            format!("Window {}", hwnd),
            if *capture_area == crate::modules::pipeline::models::graph::CaptureArea::Client {
                "client".to_string()
            } else {
                "frame".to_string()
            },
        ),
    };
    super::debug_image::DebugImageContext {
        attempt,
        frame_sequence,
        capture_target,
        capture_area,
        actions_run_afterward,
    }
}

async fn cancellable_delay(handle: &RunHandle, duration: Duration) -> bool {
    let mut active_elapsed = Duration::ZERO;
    while active_elapsed < duration {
        if stop_requested(handle) {
            return false;
        }
        if matches!(handle.get_state(), PipelineState::Paused) {
            tokio::time::sleep(Duration::from_millis(25)).await;
            continue;
        }
        let slice = (duration - active_elapsed).min(Duration::from_millis(25));
        tokio::time::sleep(slice).await;
        active_elapsed += slice;
    }
    !stop_requested(handle)
}

async fn wait_until_running(handle: &RunHandle) -> bool {
    loop {
        if stop_requested(handle) {
            return false;
        }
        if !matches!(handle.get_state(), PipelineState::Paused) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// One exclusively-owned pipeline execution. This is the aggregate root for a
/// run: mutable execution state cannot leak into command handlers or another
/// concurrent run.
struct PipelineRunSession {
    ctx: ExecutionContext,
    handle: Arc<RunHandle>,
    stream_service: Arc<StreamService>,
    app: AppHandle,
    rx: broadcast::Receiver<StreamFrame>,
    detailed_timings: bool,
    pipeline_startup_ms: Option<u64>,
    steps: Vec<StepResult>,
    next_frame_not_before: Option<Instant>,
    last_frame_sequence: Option<u64>,
    active_capture: Option<ActiveCapture>,
    was_stopped: bool,
    attempt_node: Option<String>,
    /// Quota tracking for parent-less execution (the entry node). While a
    /// children state is active the parent's quota governs; otherwise the
    /// node's own timeout bounds its attempts.
    solo_node: String,
    solo_start: Instant,
}

impl PipelineRunSession {
    fn new(
        plan: ExecutionPlan,
        registry: PluginRegistry,
        handle: Arc<RunHandle>,
        stream_service: Arc<StreamService>,
        user_params: HashMap<String, HashMap<String, serde_json::Value>>,
        save_debug_images: bool,
        detailed_timings: bool,
        pipeline_startup_ms: Option<u64>,
        app: AppHandle,
    ) -> Self {
        let mut ctx = ExecutionContext::new(plan, registry, user_params);
        ctx.save_debug_images = save_debug_images;
        let rx = stream_service.subscribe();
        Self {
            ctx, handle, stream_service, app, rx, detailed_timings, pipeline_startup_ms,
            steps: Vec::new(), next_frame_not_before: None, last_frame_sequence: None,
            active_capture: None, was_stopped: false,
            attempt_node: None, solo_node: String::new(), solo_start: Instant::now(),
        }
    }

    async fn run(self) -> EngineResult {
        // Move all run-scoped state out of the aggregate root once. No command
        // handler can access it while this state machine is executing.
        let mut ctx = self.ctx;
        let handle = self.handle;
        let stream_service = self.stream_service;
        let app = self.app;
        let mut rx = self.rx;
        let detailed_timings = self.detailed_timings;
        let pipeline_startup_ms = self.pipeline_startup_ms;
        let mut steps = self.steps;
        let mut next_frame_not_before = self.next_frame_not_before;
        let mut last_frame_sequence = self.last_frame_sequence;
        let mut active_capture = self.active_capture;
        let mut was_stopped = self.was_stopped;
        let mut attempt_node = self.attempt_node;
        let mut solo_node = self.solo_node;
        let mut solo_start = self.solo_start;

    {
        if let Ok(mut state) = handle.state.lock() {
            *state = PipelineState::Running;
        }
    }
    publish_status(&app, &handle, None);

    log::info!("[pipeline] started, entry node='{}'", ctx.current_node_id);

    // A broadcast receiver yields its oldest unread frame. After an action changes the
    // target UI, require the next node to skip frames captured before that action finished.
    'pipeline: loop {
        if let Ok(stop) = handle.should_stop.lock() {
            if *stop {
                was_stopped = true;
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

        if let Some(children) = ctx.children.as_ref() {
            if children.start_time.elapsed().as_millis() as u64 >= children.timeout_ms {
                let failed_id = match children.kind {
                    ChildrenKind::Next => children.parent_node_id.clone(),
                    ChildrenKind::Error => ctx.current_node_id.clone(),
                };
                // Quota exhausted: route the failed node's error edge under
                // the error budget with progress kept on the parent; without
                // an error edge the run fails.
                route_error(
                    &mut ctx,
                    &handle,
                    &failed_id,
                    "All children candidates timed out".to_string(),
                );
                if ctx.current_node_id.is_empty() {
                    break;
                }
                continue;
            }
        }

        let (expected_stream_target, expected_session) = match ensure_effective_target(
            ctx.graph.target.as_ref(),
            current_node.target.as_ref(),
            &stream_service,
            &app,
            &handle,
            &mut active_capture,
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                ctx.failed = true;
                ctx.failure_error = Some(format!("Node '{}': {}", current_node.id, error));
                break;
            }
        };

        if let Ok(mut node_id) = handle.current_node_id.lock() {
            *node_id = Some(current_node.id.clone());
        }
        if ctx.children.is_some() {
            if let Ok(mut child_id) = handle.children_child_id.lock() {
                *child_id = Some(current_node.id.clone());
            }
        }
        if solo_node != current_node.id {
            solo_node = current_node.id.clone();
            solo_start = Instant::now();
        }
        publish_status(&app, &handle, None);

        if !current_node.enabled {
            if step::skip(&mut ctx, &handle, &app, &mut steps, &current_node.id, false) {
                break;
            }
            continue;
        }

        let hit_count = ctx.hit_counts.get(&current_node.id).copied().unwrap_or(0);
        if hit_count >= current_node.max_hit {
            if step::skip(&mut ctx, &handle, &app, &mut steps, &current_node.id, true) {
                break;
            }
            continue;
        }

        let step_start = std::time::Instant::now();
        if attempt_node.as_deref() != Some(&current_node.id) {
            attempt_node = Some(current_node.id.clone());
            if let Ok(mut attempts) = handle.attempt_count.lock() {
                *attempts = 0;
            }
        }
        if let Ok(mut attempts) = handle.attempt_count.lock() {
            *attempts += 1;
        }
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

        if current_node.kind
            == crate::modules::pipeline::models::node::NodeKind::Entry
        {
            // Entry nodes carry no recognition or actions: the status
            // published above makes the handoff visible, then execution
            // moves straight to the entry's children.
            log::info!("[pipeline] node='{}' entry, transitioning", current_node.id);
            let step = StepResult {
                node_id: current_node.id.clone(),
                matched: true,
                confidence: 1.0,
                box_region: None,
                actions_fired: 0,
                skipped: false,
                duration_ms: step_start.elapsed().as_millis() as u64,
                error: None,
                timings_ms: detailed_timings.then_some(timings),
            };
            step::record(&mut ctx, &handle, &app, &mut steps, step);

            let transition = routing::next(&ctx);
            routing::apply(&mut ctx, &handle, transition);
            if ctx.current_node_id.is_empty() {
                break;
            }
            continue;
        }

        let phase_start = std::time::Instant::now();
        let captured_frame = if stream_service.is_streaming() {
            match frame::receive_after(
                &stream_service,
                &mut rx,
                next_frame_not_before,
                last_frame_sequence,
                Duration::from_secs(3),
                Some(expected_session),
                Some(&expected_stream_target),
                Some(&handle),
            )
            .await
            {
                Ok((frame, stale_frames_skipped, _paused)) => {
                    next_frame_not_before = None;
                    last_frame_sequence = Some(frame.sequence);
                    if stale_frames_skipped > 0 {
                        log::debug!(
                            "[pipeline] node='{}' skipped {} frame(s) captured before the previous actions completed",
                            current_node.id,
                            stale_frames_skipped
                        );
                    }
                    image::load_from_memory(&frame.bytes)
                        .ok()
                        .map(|image| (image, frame.bounds, frame.sequence))
                }
                Err((e, _paused)) => {
                    log::warn!(
                        "[pipeline] node='{}' stream recv error: {}",
                        current_node.id,
                        e
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

        let (screenshot, frame_bounds, frame_sequence) = match captured_frame {
            Some(frame) => frame,
            None => {
                if !quota_exceeded(&ctx, &current_node, &solo_start) {
                    let wait_ms = current_node
                        .rate_limit
                        .saturating_sub(step_start.elapsed().as_millis() as u64);
                    if !cancellable_delay(&handle, Duration::from_millis(wait_ms)).await {
                        was_stopped = true;
                        break 'pipeline;
                    }
                    next_frame_not_before = Some(Instant::now());
                    continue;
                }
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
                    skipped: false,
                    duration_ms: step_start.elapsed().as_millis() as u64,
                    error: Some("Failed to capture screen".to_string()),
                    timings_ms: detailed_timings.then_some(timings),
                };
                step::record(&mut ctx, &handle, &app, &mut steps, step);

                let failed_id = quota_owner(&ctx, &current_node);
                route_error(
                    &mut ctx,
                    &handle,
                    &failed_id,
                    "Failed to capture a fresh stream frame".to_string(),
                );
                if ctx.current_node_id.is_empty() {
                    break;
                }
                continue;
            }
        };

        // A cached frame normally carries the exact capture bounds. Resolve a
        // fallback only for legacy producers so action geometry always has an
        // explicit containment boundary.
        let resolved_frame_bounds = frame_bounds.or_else(|| match &expected_stream_target {
            StreamTarget::Window { hwnd, capture_area } => {
                crate::modules::pipeline::plugins::shared::window_capture_bounds(
                    *hwnd,
                    *capture_area,
                    screenshot.width(),
                    screenshot.height(),
                )
                .ok()
            }
            StreamTarget::Screen { index } => {
                crate::modules::pipeline::plugins::shared::screen_capture_bounds(*index).ok()
            }
        });

        let phase_start = std::time::Instant::now();
        let node_params = ctx
            .user_params
            .get(&current_node.id)
            .cloned()
            .unwrap_or_default();
        let effective_roi = crate::modules::pipeline::plugins::shared::resolve_node_recognition_roi(
            current_node.recognition.roi.as_ref(),
            current_node.recognition.roi_offset.as_ref(),
            screenshot.width(),
            screenshot.height(),
            resolved_frame_bounds,
            &expected_stream_target,
            &ctx.recognized_regions,
            &ctx.recognized_region_targets,
        );
        let substituted_recognition_kind = current_node.recognition.kind.substitute(&node_params);
        let match_result = match effective_roi.as_ref() {
            Ok(roi) => ctx.registry.evaluate_recognition_with_selection(
                &screenshot,
                &substituted_recognition_kind,
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
                if ctx.save_debug_images {
                    let phase_start = std::time::Instant::now();
                    let attempt = handle.attempt_count.lock().map(|value| *value).unwrap_or(0);
                    let failed_match =
                        crate::modules::pipeline::models::recognizer::MatchResult::default();
                    log::info!(
                        "[pipeline] node='{}' saving debug image (attempt={}, recognition_error=true)",
                        current_node.id,
                        attempt
                    );
                    if let Err(save_error) = super::debug_image::save_debug_image(
                        &screenshot,
                        &current_node.id,
                        handle.step_count.lock().map(|count| *count).unwrap_or(0),
                        &current_node.recognition.kind,
                        effective_roi.as_ref().ok(),
                        &failed_match,
                        &[],
                        &std::collections::HashMap::new(),
                        Some(&debug_image_context(
                            &expected_stream_target,
                            attempt,
                            frame_sequence,
                            false,
                        )),
                    ) {
                        log::warn!(
                            "[pipeline] node='{}' attempt={} failed to save recognition-error debug image: {}",
                            current_node.id,
                            attempt,
                            save_error
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
                    confidence: 0.0,
                    box_region: None,
                    actions_fired: 0,
                    skipped: false,
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
                step::record(&mut ctx, &handle, &app, &mut steps, step);

                let failed_id = current_node.id.clone();
                route_error(&mut ctx, &handle, &failed_id, format!("Recognition error: {}", e));
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
        let mut box_mapped_to_screen = true;
        if let Some(region) = match_result.box_region.as_mut() {
            let local = region.clone();
            let converted = if let Some(bounds) = resolved_frame_bounds {
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
                Err(error) => {
                    box_mapped_to_screen = false;
                    log::warn!(
                        "[pipeline] node='{}' coordinate conversion failed: {}",
                        current_node.id,
                        error
                    );
                }
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
            // Persist every recognition attempt, not only the final attempt that
            // exhausts timeout. This makes transient misses and changing target
            // content visible in chronological debug output.
            if ctx.save_debug_images {
                let phase_start = std::time::Instant::now();
                let attempt = handle.attempt_count.lock().map(|value| *value).unwrap_or(0);
                log::info!(
                    "[pipeline] node='{}' saving debug image (attempt={}, matched=false)",
                    current_node.id,
                    attempt
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
                    Some(&debug_image_context(
                        &expected_stream_target,
                        attempt,
                        frame_sequence,
                        false,
                    )),
                ) {
                    log::warn!(
                        "[pipeline] node='{}' attempt={} failed to save debug image: {}",
                        current_node.id,
                        attempt,
                        e
                    );
                }
                let debug_image_elapsed = phase_start.elapsed();
                // Wall-clock parent quota includes diagnostic I/O.
                if detailed_timings {
                    timings.insert(
                        "debug_image".to_string(),
                        debug_image_elapsed.as_millis() as u64,
                    );
                }
            }

            // Single-level quota: each miss rotates to the next candidate
            // (`n == 1` re-selects the only child) until the parent quota
            // is exhausted.
            if !quota_exceeded(&ctx, &current_node, &solo_start) {
                let wait_ms = current_node
                    .rate_limit
                    .saturating_sub(step_start.elapsed().as_millis() as u64);
                if !cancellable_delay(&handle, Duration::from_millis(wait_ms)).await {
                    was_stopped = true;
                    break 'pipeline;
                }
                next_frame_not_before = Some(Instant::now());
                if let Some(children) = ctx.children.as_mut() {
                    children.current_index =
                        (children.current_index + 1) % children.candidates.len();
                    ctx.current_node_id = children.candidates[children.current_index].clone();
                }
                continue;
            }
            log::info!(
                "[pipeline] node='{}' not matched, confidence={:.3}",
                current_node.id,
                match_result.confidence
            );

            let quota = quota_ms(&ctx, &current_node);
            let step = StepResult {
                node_id: current_node.id.clone(),
                matched: false,
                confidence: match_result.confidence,
                box_region: None,
                actions_fired: 0,
                skipped: false,
                duration_ms: step_start.elapsed().as_millis() as u64,
                error: Some(match_result.selection_error.clone().unwrap_or_else(|| {
                    format!("Recognition timed out after {}ms", quota)
                })),
                timings_ms: detailed_timings.then_some(timings),
            };
            step::record(&mut ctx, &handle, &app, &mut steps, step);

            let failed_id = quota_owner(&ctx, &current_node);
            route_error(
                &mut ctx,
                &handle,
                &failed_id,
                match_result
                    .selection_error
                    .clone()
                    .unwrap_or_else(|| "Recognition timed out without a match".to_string()),
            );
            if ctx.current_node_id.is_empty() {
                break;
            }
            continue;
        }


        if let Some(region) = match_result
            .box_region
            .as_ref()
            .filter(|_| box_mapped_to_screen)
        {
            ctx.recognized_regions
                .insert(current_node.id.clone(), region.clone());
            ctx.recognized_region_targets
                .insert(current_node.id.clone(), expected_stream_target.clone());
        }
        if let Some(region) = &debug_match_result.box_region {
            ctx.debug_recognized_regions
                .insert(current_node.id.clone(), region.clone());
        }
        *ctx.hit_counts.entry(current_node.id.clone()).or_insert(0) += 1;
        if let Ok(mut hc) = handle.hit_counts.lock() {
            *hc = ctx.hit_counts.clone();
        }

        log::info!(
            "[pipeline] node='{}' matched, confidence={:.3}{}",
            current_node.id,
            match_result.confidence,
            match &match_result.text {
                Some(t) => format!(", text=\"{}\"", t),
                None => String::new(),
            }
        );

        let substituted_actions = substitute_actions(&current_node.actions, &node_params);
        let mut actions_fired = 0;
        let mapped_actions = resolved_frame_bounds
            .ok_or_else(|| {
                crate::modules::pipeline::plugins::PluginError::ActionFailed(
                    "Capture bounds are unavailable; refusing to execute an unbounded action"
                        .to_string(),
                )
            })
            .and_then(|bounds| {
                substituted_actions
                    .iter()
                    .map(|action| {
                        crate::modules::pipeline::plugins::shared::map_action_checked(
                            action,
                            debug_match_result.box_region.as_ref(),
                            &ctx.debug_recognized_regions,
                            screenshot.width(),
                            screenshot.height(),
                            bounds,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
            });
        let (mapped_actions, mut action_error) = match mapped_actions {
            Ok(actions) => (actions, None),
            Err(error) => (Vec::new(), Some(format!("Action error: {}", error))),
        };

        let phase_start = std::time::Instant::now();
        if action_error.is_none()
            && !cancellable_delay(&handle, Duration::from_millis(current_node.pre_delay)).await
        {
            was_stopped = true;
            break 'pipeline;
        }
        if detailed_timings {
            timings.insert(
                "pre_delay".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }

        let phase_start = std::time::Instant::now();
        for repetition in 0..current_node.repeat {
            if action_error.is_some() {
                break;
            }
            if repetition > 0
                && !cancellable_delay(&handle, Duration::from_millis(current_node.repeat_delay))
                    .await
            {
                was_stopped = true;
                break 'pipeline;
            }
            for action in &mapped_actions {
                if !wait_until_running(&handle).await {
                    was_stopped = true;
                    break 'pipeline;
                }
                if let StreamTarget::Window { hwnd, .. } = &expected_stream_target {
                    if let Err(error) =
                        crate::repositories::window::require_window_foreground(*hwnd)
                    {
                        action_error =
                            Some(format!("Action error: target_not_foreground: {error}"));
                        break;
                    }
                }
                let result = ctx
                    .registry
                    .execute_action(action, None, &std::collections::HashMap::new())
                    .await;
                match result {
                    Ok(()) => {
                        actions_fired += 1;
                    }
                    Err(e) => {
                        action_error = Some(format!("Action error: {}", e));
                        break;
                    }
                }

                if action_error.is_some() {
                    break;
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
        if action_error.is_none()
            && !cancellable_delay(&handle, Duration::from_millis(current_node.post_delay)).await
        {
            was_stopped = true;
            break 'pipeline;
        }
        if detailed_timings {
            timings.insert(
                "post_delay".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }
        if actions_fired > 0 {
            // Honor post_delay before defining the earliest acceptable frame for the
            // next node. A frame produced during the delay can still contain UI state
            // from before the target application handled the action.
            next_frame_not_before = Some(Instant::now());
        }

        log::info!(
            "[pipeline] node='{}' actions fired: {}",
            current_node.id,
            actions_fired
        );

        // A matched attempt is persisted after action execution so its overlay
        // and sidebar can report which actions actually ran afterward.
        if ctx.save_debug_images {
            let phase_start = std::time::Instant::now();
            let attempt = handle.attempt_count.lock().map(|value| *value).unwrap_or(0);
            log::info!(
                "[pipeline] node='{}' saving debug image (attempt={}, matched=true, actions={})",
                current_node.id,
                attempt,
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
                Some(&debug_image_context(
                    &expected_stream_target,
                    attempt,
                    frame_sequence,
                    actions_fired > 0,
                )),
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
            skipped: false,
            duration_ms: step_start.elapsed().as_millis() as u64,
            error: action_error.clone(),
            timings_ms: detailed_timings.then_some(timings),
        };
        step::record(&mut ctx, &handle, &app, &mut steps, step);

        if action_error.is_some() {
            let failed_id = current_node.id.clone();
            route_error(
                &mut ctx,
                &handle,
                &failed_id,
                action_error
                    .clone()
                    .unwrap_or_else(|| "Action failed".to_string()),
            );
            if ctx.current_node_id.is_empty() {
                break;
            }
            continue;
        }

        let transition = routing::next(&ctx);
        routing::apply(&mut ctx, &handle, transition);
        if ctx.current_node_id.is_empty() {
            break;
        }
    }

    let final_state = if was_stopped {
        PipelineState::Stopped
    } else if ctx.failed {
        PipelineState::Failed(
            ctx.failure_error
                .clone()
                .unwrap_or_else(|| "Pipeline failed".to_string()),
        )
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
    if let Ok(mut node_id) = handle.current_node_id.lock() {
        *node_id = None;
    }
    publish_status(&app, &handle, None);

    match &final_state {
        PipelineState::Failed(error) => log::error!(
            "[pipeline] FAILED: error='{}', steps={}, last_node={}",
            error,
            steps.len(),
            ctx.step_results
                .last()
                .map(|step| step.node_id.as_str())
                .unwrap_or("<none>")
        ),
        PipelineState::Stopped => log::warn!(
            "[pipeline] STOPPED: steps={}, last_node={}",
            steps.len(),
            ctx.step_results
                .last()
                .map(|step| step.node_id.as_str())
                .unwrap_or("<none>")
        ),
        PipelineState::Completed => log::info!("[pipeline] completed: steps={}", steps.len()),
        state => log::warn!(
            "[pipeline] ended in unexpected state={state:?}, steps={}",
            steps.len()
        ),
    }

    EngineResult {
        success: final_state == PipelineState::Completed,
        steps,
        error: match final_state {
            PipelineState::Failed(msg) => Some(msg),
            _ => None,
        },
    }
    }

}

/// Public command-facing entry point. Construction is deliberately separate
/// from execution so callers cannot observe a partially initialized session.
pub async fn run_pipeline(
    plan: ExecutionPlan,
    registry: PluginRegistry,
    handle: Arc<RunHandle>,
    stream_service: Arc<StreamService>,
    user_params: HashMap<String, HashMap<String, serde_json::Value>>,
    save_debug_images: bool,
    detailed_timings: bool,
    pipeline_startup_ms: Option<u64>,
    app: AppHandle,
) -> EngineResult {
    PipelineRunSession::new(
        plan, registry, handle, stream_service, user_params, save_debug_images,
        detailed_timings, pipeline_startup_ms, app,
    ).run().await
}

/// Route the failed node's error edge as a single-candidate children state
/// (error budget, progress kept on the failed node). Without an error edge
/// the run fails.
fn route_error(
    ctx: &mut ExecutionContext,
    handle: &RunHandle,
    failed_node_id: &str,
    reason: String,
) {
    match routing::error(ctx, failed_node_id, reason) {
        routing::AttemptOutcome::Transition(transition) => {
            routing::apply(ctx, handle, transition);
        }
        routing::AttemptOutcome::Fail { reason } => {
            log::warn!("[pipeline] {reason}");
            ctx.failed = true;
            ctx.failure_error = Some(reason);
            ctx.current_node_id.clear();
        }
        _ => unreachable!("error routing only yields transition or fail"),
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
        Action::ClickKey { key } => Action::ClickKey {
            key: key.iter().map(|v| substitute_json(v, params)).collect(),
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
    use crate::services::stream::StreamTarget;

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

    #[tokio::test]
    async fn skips_frames_captured_before_previous_actions_completed() {
        let (sender, mut receiver) = broadcast::channel(4);
        let stale = StreamFrame {
            bytes: vec![1],
            width: 1,
            height: 1,
            target: StreamTarget::Screen { index: 0 },
            bounds: None,
            captured_at: Instant::now(),
            session_id: 1,
            sequence: 1,
        };
        sender.send(stale).unwrap();

        let actions_completed_at = Instant::now();
        let fresh = StreamFrame {
            bytes: vec![2],
            width: 1,
            height: 1,
            target: StreamTarget::Screen { index: 0 },
            bounds: None,
            captured_at: Instant::now(),
            session_id: 1,
            sequence: 2,
        };
        sender.send(fresh).unwrap();

        let stream = StreamService::new();
        let (received, skipped, _) = frame::receive_after(
            &stream,
            &mut receiver,
            Some(actions_completed_at),
            None,
            Duration::from_millis(100),
            Some(1),
            Some(&StreamTarget::Screen { index: 0 }),
            None,
        )
        .await
        .unwrap();

        assert_eq!(received.bytes, vec![2]);
        assert_eq!(skipped, 1);
    }

    #[tokio::test]
    async fn pause_suspends_and_stop_cancels_delays() {
        let paused = Arc::new(RunHandle::new("paused".to_string()));
        *paused.state.lock().unwrap() = PipelineState::Paused;
        let paused_task = {
            let handle = paused.clone();
            tokio::spawn(async move { cancellable_delay(&handle, Duration::from_millis(20)).await })
        };
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(!paused_task.is_finished());
        *paused.state.lock().unwrap() = PipelineState::Running;
        assert!(paused_task.await.unwrap());

        let stopped = Arc::new(RunHandle::new("stopped".to_string()));
        *stopped.state.lock().unwrap() = PipelineState::Running;
        let stopped_task = {
            let handle = stopped.clone();
            tokio::spawn(async move { cancellable_delay(&handle, Duration::from_secs(1)).await })
        };
        tokio::time::sleep(Duration::from_millis(25)).await;
        stopped.stop();
        assert!(!stopped_task.await.unwrap());
        assert_eq!(stopped.get_state(), PipelineState::Stopping);
    }
}

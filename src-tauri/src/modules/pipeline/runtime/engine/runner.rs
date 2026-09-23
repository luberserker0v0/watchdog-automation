mod parameters;
mod patrol_worker;
mod target;

use target::ensure_effective_target;

pub(crate) use parameters::substitute_actions;
use patrol_worker::{
    pending_action_for_snapshot, sample_patrol, PatrolActionFrames, PatrolRecoveryReport,
    PatrolSampler, PatrolSnapshot, PatrolWorker,
};

use super::frame;
use super::freeze::{self, FreezeOutcome};
use super::patrol_state::{DispatchOutcome, RecoveryEvidence, ScreenObservation};
use super::routing;
use super::step;
use super::{ChildrenKind, EngineResult, ExecutionContext, PipelineState, RunHandle, StepResult};
use crate::modules::capture::services::stream::{StreamFrame, StreamService, StreamTarget};
use crate::modules::pipeline::authoring::models::node::Node;
use crate::modules::pipeline::plugins::PluginRegistry;
use crate::modules::pipeline::runtime::chat::{cache_image, ChatContent, PatrolChatSink};
use crate::modules::pipeline::runtime::patrol::{
    PatrolRuntime, RecoveryDecision, RecoveryInput, RecoveryPolicy,
};
use crate::modules::pipeline::runtime::plan::ExecutionPlan;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

#[derive(Clone)]
pub(super) struct ActiveCapture {
    pub(super) configured_key: String,
    pub(super) stream_target: StreamTarget,
    pub(super) session_id: u64,
}

fn publish_status(app: &AppHandle, handle: &RunHandle, appended_step: Option<StepResult>) {
    // Delivery is best-effort: `get_pipeline_status` remains the recovery path
    // when a window was not listening while an event was emitted.
    let _ = app.emit("pipeline-status", handle.event(appended_step));
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
fn quota_exceeded(ctx: &ExecutionContext, current_node: &Node, solo_start: &Instant) -> bool {
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
        Some(children) if children.kind == ChildrenKind::Next => children.parent_node_id.clone(),
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
            if *capture_area
                == crate::modules::pipeline::authoring::models::graph::CaptureArea::Client
            {
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
    patrol: Option<PatrolRuntime>,
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
        patrol: Option<PatrolRuntime>,
        app: AppHandle,
    ) -> Self {
        let mut ctx = ExecutionContext::new(plan, registry, user_params);
        ctx.save_debug_images = save_debug_images;
        let rx = stream_service.subscribe();
        Self {
            ctx,
            handle,
            stream_service,
            app,
            rx,
            detailed_timings,
            pipeline_startup_ms,
            patrol,
            steps: Vec::new(),
            next_frame_not_before: None,
            last_frame_sequence: None,
            active_capture: None,
            was_stopped: false,
            attempt_node: None,
            solo_node: String::new(),
            solo_start: Instant::now(),
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
        let patrol = self.patrol.map(Arc::new);
        let mut reference_tasks = Vec::new();
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
        let mut patrol_worker = patrol.as_ref().map(|patrol| {
            PatrolWorker::start(
                Arc::clone(patrol),
                ctx.graph.clone(),
                Arc::clone(&handle),
                app.clone(),
            )
        });
        let mut redirect_count = 0u32;
        let mut redirect_routes = HashSet::<(String, String)>::new();

        // A broadcast receiver yields its oldest unread frame. After an action changes the
        // target UI, require the next node to skip frames captured before that action finished.
        'execution: loop {
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

                if matches!(
                    current_node.kind,
                    crate::modules::pipeline::authoring::models::node::NodeKind::Entry
                        | crate::modules::pipeline::authoring::models::node::NodeKind::Endpoint
                ) {
                    // Structural nodes record a step without capture, recognition,
                    // or actions. Routing decides whether this is a completion.
                    log::info!(
                        "[pipeline] node='{}' structural, transitioning",
                        current_node.id
                    );
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
                                .map(|image| (Arc::new(image), frame.bounds, frame.sequence))
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

                let (mut screenshot, mut frame_bounds, mut frame_sequence) = match captured_frame {
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

                let capture_target = format!("{expected_stream_target:?}");
                if let Some(worker) = patrol_worker.as_ref() {
                    worker.observe(PatrolSnapshot {
                        node_id: current_node.id.clone(),
                        screenshot: screenshot.clone(),
                        anchor_confirmed: false,
                        capture_target: capture_target.clone(),
                        capture_session: expected_session,
                        frame_sequence,
                    });
                }
                if patrol.is_some() {
                    if let Ok(mut state) = handle.patrol_state.lock() {
                        state.observe(ScreenObservation {
                            expected_node_id: current_node.id.clone(),
                            target: capture_target.clone(),
                            session: expected_session,
                            frame: frame_sequence,
                            elapsed_ms: handle.start_time.elapsed().as_millis() as u64,
                            recognizer_matched: None,
                            recognizer_confidence: None,
                        });
                    }
                }

                // A cached frame normally carries the exact capture bounds. Resolve a
                // fallback only for legacy producers so action geometry always has an
                // explicit containment boundary.
                let mut resolved_frame_bounds =
                    frame_bounds.or_else(|| match &expected_stream_target {
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
                            crate::modules::pipeline::plugins::shared::screen_capture_bounds(*index)
                                .ok()
                        }
                    });

                let phase_start = std::time::Instant::now();
                let node_params = ctx
                    .user_params
                    .get(&current_node.id)
                    .cloned()
                    .unwrap_or_default();
                let mut effective_roi =
                    crate::modules::pipeline::plugins::shared::resolve_node_recognition_roi(
                        current_node.recognition.roi.as_ref(),
                        current_node.recognition.roi_offset.as_ref(),
                        screenshot.width(),
                        screenshot.height(),
                        resolved_frame_bounds,
                        &expected_stream_target,
                        &ctx.recognized_regions,
                        &ctx.recognized_region_targets,
                    );
                let substituted_recognition_kind =
                    current_node.recognition.kind.substitute(&node_params);
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
                            let attempt =
                                handle.attempt_count.lock().map(|value| *value).unwrap_or(0);
                            let failed_match =
                        crate::modules::pipeline::authoring::models::recognizer::MatchResult::default();
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
                        route_error(
                            &mut ctx,
                            &handle,
                            &failed_id,
                            format!("Recognition error: {}", e),
                        );
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
                let mut debug_match_result = match_result.clone();

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
                    Some(crate::modules::pipeline::authoring::models::graph::PipelineTarget::Window {
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
                    Some(crate::modules::pipeline::authoring::models::graph::PipelineTarget::Screen {
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

                let matched =
                    crate::modules::pipeline::authoring::models::recognizer::effective_match(
                        &match_result,
                        current_node.inverse,
                    );
                if patrol.is_some() {
                    if let Ok(mut state) = handle.patrol_state.lock() {
                        state.recognize(
                            &current_node.id,
                            expected_session,
                            frame_sequence,
                            matched,
                            match_result.confidence,
                        );
                    }
                }

                if let Some(worker) = patrol_worker.as_ref() {
                    worker.set_anchor_confirmed(
                    frame_sequence,
                    matched
                        && !current_node.inverse
                        && !matches!(
                            &current_node.recognition.kind,
                            crate::modules::pipeline::authoring::models::recognizer::Recognition::DirectHit
                        ),
                );
                }

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
                            ctx.current_node_id =
                                children.candidates[children.current_index].clone();
                        }
                        continue;
                    }
                    log::info!(
                        "[pipeline] node='{}' not matched, confidence={:.3}",
                        current_node.id,
                        match_result.confidence
                    );

                    let quota = quota_ms(&ctx, &current_node);
                    let step =
                        StepResult {
                            node_id: current_node.id.clone(),
                            matched: false,
                            confidence: match_result.confidence,
                            box_region: None,
                            actions_fired: 0,
                            skipped: false,
                            duration_ms: step_start.elapsed().as_millis() as u64,
                            error: Some(match_result.selection_error.clone().unwrap_or_else(
                                || format!("Recognition timed out after {}ms", quota),
                            )),
                            timings_ms: detailed_timings.then_some(timings),
                        };
                    step::record(&mut ctx, &handle, &app, &mut steps, step);

                    let failed_id = quota_owner(&ctx, &current_node);
                    let timeout_reason = match (&match_result.selection_error, &ctx.children) {
                        (Some(error), _) => error.clone(),
                        (None, Some(children)) if children.kind == ChildrenKind::Next => format!(
                            "All child candidates timed out; last candidate '{}' did not match",
                            current_node.id
                        ),
                        (None, _) => "Recognition timed out without a match".to_string(),
                    };
                    route_error(&mut ctx, &handle, &failed_id, timeout_reason);
                    if ctx.current_node_id.is_empty() {
                        break;
                    }
                    continue;
                }

                let pre_wait_enabled =
                    !matches!(current_node.pre_wait_freezes.settings(), Ok(None));
                if pre_wait_enabled {
                    let phase_start = Instant::now();
                    let mut pre_error = None;
                    let wait_outcome = freeze::wait(
                        &stream_service,
                        &mut rx,
                        &current_node.pre_wait_freezes,
                        expected_session,
                        &expected_stream_target,
                        Some((&screenshot, frame_sequence)),
                        &handle,
                        Some(if let Some(children) = ctx.children.as_ref() {
                            Duration::from_millis(children.timeout_ms)
                                .saturating_sub(children.start_time.elapsed())
                        } else {
                            Duration::from_millis(current_node.timeout)
                                .saturating_sub(solo_start.elapsed())
                        }),
                    )
                    .await;
                    if detailed_timings {
                        timings.insert(
                            "pre_wait_freezes".into(),
                            phase_start.elapsed().as_millis() as u64,
                        );
                    }
                    match wait_outcome {
                        FreezeOutcome::Stopped => {
                            was_stopped = true;
                            break 'pipeline;
                        }
                        FreezeOutcome::Failed(error) => {
                            pre_error = Some(format!("Pre-action screen stability: {error}"))
                        }
                        FreezeOutcome::Stable(frame) => {
                            log::info!(
                            "[pipeline] node='{}' pre-action screen stable after {}ms (frame={})",
                            current_node.id,
                            phase_start.elapsed().as_millis(),
                            frame.sequence
                        );
                            let delay_started = Instant::now();
                            if !cancellable_delay(
                                &handle,
                                Duration::from_millis(current_node.pre_delay),
                            )
                            .await
                            {
                                was_stopped = true;
                                break 'pipeline;
                            }
                            if detailed_timings {
                                timings.insert(
                                    "pre_delay".into(),
                                    delay_started.elapsed().as_millis() as u64,
                                );
                            }
                            let fresh_frame = match frame::receive_after(
                                &stream_service,
                                &mut rx,
                                Some(Instant::now()),
                                Some(frame.sequence),
                                Duration::from_secs(2),
                                Some(expected_session),
                                Some(&expected_stream_target),
                                Some(&handle),
                            )
                            .await
                            {
                                Ok((fresh, _, _)) => Some(fresh),
                                Err((error, _)) if error.contains("pipeline stopped") => {
                                    was_stopped = true;
                                    break 'pipeline;
                                }
                                Err((error, _)) => {
                                    pre_error =
                                        Some(format!("No fresh frame after pre-delay: {error}"));
                                    None
                                }
                            };
                            if let Some(frame) = fresh_frame {
                                match image::load_from_memory(&frame.bytes) {
                                    Ok(image) => {
                                        screenshot = Arc::new(image);
                                        frame_bounds = frame.bounds;
                                        frame_sequence = frame.sequence;
                                        last_frame_sequence = Some(frame.sequence);
                                        if let Some(worker) = patrol_worker.as_ref() {
                                            worker.observe(PatrolSnapshot {
                                                node_id: current_node.id.clone(),
                                                screenshot: Arc::clone(&screenshot),
                                                anchor_confirmed: false,
                                                capture_target: capture_target.clone(),
                                                capture_session: expected_session,
                                                frame_sequence,
                                            });
                                        }
                                        resolved_frame_bounds = frame_bounds.or_else(|| match &expected_stream_target {
                                    StreamTarget::Window { hwnd, capture_area } =>
                                        crate::modules::pipeline::plugins::shared::window_capture_bounds(*hwnd, *capture_area, screenshot.width(), screenshot.height()).ok(),
                                    StreamTarget::Screen { index } =>
                                        crate::modules::pipeline::plugins::shared::screen_capture_bounds(*index).ok(),
                                });
                                        effective_roi = crate::modules::pipeline::plugins::shared::resolve_node_recognition_roi(
                                    current_node.recognition.roi.as_ref(),
                                    current_node.recognition.roi_offset.as_ref(),
                                    screenshot.width(), screenshot.height(), resolved_frame_bounds,
                                    &expected_stream_target, &ctx.recognized_regions, &ctx.recognized_region_targets,
                                );
                                        let refreshed = effective_roi
                                            .as_ref()
                                            .map_err(Clone::clone)
                                            .and_then(|roi| {
                                                ctx.registry.evaluate_recognition_with_selection(
                                                    &screenshot,
                                                    &substituted_recognition_kind,
                                                    Some(roi),
                                                    current_node
                                                        .recognition
                                                        .candidate_selection
                                                        .as_ref(),
                                                )
                                            });
                                        match refreshed {
                                    Ok(fresh) if crate::modules::pipeline::authoring::models::recognizer::effective_match(&fresh, current_node.inverse) => {
                                        debug_match_result = fresh.clone();
                                        match_result = fresh;
                                        if let Some(worker) = patrol_worker.as_ref() {
                                            worker.set_anchor_confirmed(frame_sequence, !current_node.inverse && !matches!(
                                                &current_node.recognition.kind,
                                                crate::modules::pipeline::authoring::models::recognizer::Recognition::DirectHit
                                            ));
                                        }
                                        if patrol.is_some() {
                                            if let Ok(mut state) = handle.patrol_state.lock() {
                                                state.recognize(&current_node.id, expected_session, frame_sequence, true, match_result.confidence);
                                            }
                                        }
                                        if let Some(region) = match_result.box_region.as_mut() {
                                            if let Some(bounds) = resolved_frame_bounds {
                                                *region = crate::modules::pipeline::plugins::shared::map_region(region, screenshot.width(), screenshot.height(), bounds);
                                                box_mapped_to_screen = true;
                                            } else {
                                                pre_error = Some("Pre-action capture bounds unavailable".into());
                                            }
                                        }
                                    }
                                    Ok(_) => pre_error = Some("Target no longer matches after screen settled".into()),
                                    Err(error) => pre_error = Some(format!("Pre-action recognition failed: {error}")),
                                }
                                    }
                                    Err(error) => {
                                        pre_error =
                                            Some(format!("Pre-action frame decode failed: {error}"))
                                    }
                                }
                            }
                        }
                    }
                    if pre_error.is_none() && quota_exceeded(&ctx, &current_node, &solo_start) {
                        pre_error = Some("Node timeout elapsed before action dispatch".into());
                    }
                    if let Some(error) = pre_error {
                        next_frame_not_before = Some(Instant::now());
                        let step = StepResult {
                            node_id: current_node.id.clone(),
                            matched: false,
                            confidence: 0.0,
                            box_region: None,
                            actions_fired: 0,
                            skipped: false,
                            duration_ms: step_start.elapsed().as_millis() as u64,
                            error: Some(error.clone()),
                            timings_ms: detailed_timings.then_some(timings),
                        };
                        step::record(&mut ctx, &handle, &app, &mut steps, step);
                        let failed_id = if quota_exceeded(&ctx, &current_node, &solo_start) {
                            quota_owner(&ctx, &current_node)
                        } else {
                            current_node.id.clone()
                        };
                        route_error(&mut ctx, &handle, &failed_id, error);
                        if ctx.current_node_id.is_empty() {
                            break;
                        }
                        continue;
                    }
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
                    && !pre_wait_enabled
                    && !cancellable_delay(&handle, Duration::from_millis(current_node.pre_delay))
                        .await
                {
                    was_stopped = true;
                    break 'pipeline;
                }
                if detailed_timings && !pre_wait_enabled {
                    timings.insert(
                        "pre_delay".to_string(),
                        phase_start.elapsed().as_millis() as u64,
                    );
                }

                let action_attempt_id = if !mapped_actions.is_empty() {
                    if let Some(worker) = patrol_worker.as_ref() {
                        let expected_successors = ctx
                        .plan
                        .next(&current_node.id)
                        .iter()
                        .filter(|node_id| {
                            ctx.graph.find_node(node_id).is_some_and(|node| {
                                node.kind
                                    == crate::modules::pipeline::authoring::models::node::NodeKind::Normal
                            })
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                        let error_successor = ctx.plan.error(&current_node.id).and_then(|node_id| {
                        ctx.graph
                            .find_node(node_id)
                            .filter(|node| {
                                node.kind
                                    == crate::modules::pipeline::authoring::models::node::NodeKind::Normal
                            })
                            .map(|_| node_id.to_string())
                    });
                        {
                            let elapsed_ms = handle.start_time.elapsed().as_millis() as u64;
                            let attempt_id = handle.patrol_state.lock().ok().map(|mut state| {
                                state.begin_attempt(
                                    &current_node.id,
                                    expected_successors.clone(),
                                    error_successor.clone(),
                                    &capture_target,
                                    expected_session,
                                    frame_sequence,
                                    elapsed_ms,
                                )
                            });
                            if let Some(attempt_id) = attempt_id {
                                let actions = mapped_actions
                                    .iter()
                                    .map(|action| {
                                        let (kind, intent) =
                                            super::patrol_state::summarize_action(action);
                                        if intent.is_empty() {
                                            kind.to_string()
                                        } else {
                                            format!("{kind} {intent}")
                                        }
                                    })
                                    .collect();
                                worker.record_action_attempt(PatrolActionFrames {
                                    attempt_id,
                                    node_id: current_node.id.clone(),
                                    actions,
                                    expected_successors,
                                    error_successor,
                                    before: Arc::clone(&screenshot),
                                    target: capture_target.clone(),
                                    session: expected_session,
                                    before_frame: frame_sequence,
                                });
                            }
                            attempt_id
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let phase_start = std::time::Instant::now();
                for repetition in 0..current_node.repeat {
                    if action_error.is_some() {
                        break;
                    }
                    if repetition > 0
                        && !cancellable_delay(
                            &handle,
                            Duration::from_millis(current_node.repeat_delay),
                        )
                        .await
                    {
                        was_stopped = true;
                        break 'pipeline;
                    }
                    for (action_index, action) in mapped_actions.iter().enumerate() {
                        if !wait_until_running(&handle).await {
                            was_stopped = true;
                            break 'pipeline;
                        }
                        let evidence_id = action_attempt_id.and_then(|attempt_id| {
                            handle.patrol_state.lock().ok().map(|mut state| {
                                state.begin_action(
                                    attempt_id,
                                    &current_node.id,
                                    repetition,
                                    action_index,
                                    action,
                                    &capture_target,
                                    expected_session,
                                    frame_sequence,
                                    handle.start_time.elapsed().as_millis() as u64,
                                )
                            })
                        });
                        if let StreamTarget::Window { hwnd, .. } = &expected_stream_target {
                            let hwnd = *hwnd;
                            let foreground_result = tokio::task::spawn_blocking(move || {
                            crate::modules::capture::repositories::window::ensure_window_foreground(
                                hwnd,
                            )
                        })
                        .await;
                            let foreground_error = match foreground_result {
                            Ok(Ok(_)) => crate::modules::capture::repositories::window::require_window_foreground(hwnd).err(),
                            Ok(Err(error)) => Some(error),
                            Err(error) => Some(format!("Target foreground worker failed: {error}")),
                        };
                            if let Some(error) = foreground_error {
                                if let Some(evidence_id) = evidence_id {
                                    if let Ok(mut state) = handle.patrol_state.lock() {
                                        state.finish_action(
                                            evidence_id,
                                            DispatchOutcome::Blocked(format!(
                                                "target_not_foreground: {error}"
                                            )),
                                            handle.start_time.elapsed().as_millis() as u64,
                                        );
                                    }
                                }
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
                                if let Some(evidence_id) = evidence_id {
                                    if let Ok(mut state) = handle.patrol_state.lock() {
                                        state.finish_action(
                                            evidence_id,
                                            DispatchOutcome::Sent,
                                            handle.start_time.elapsed().as_millis() as u64,
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                if let Some(evidence_id) = evidence_id {
                                    if let Ok(mut state) = handle.patrol_state.lock() {
                                        state.finish_action(
                                            evidence_id,
                                            DispatchOutcome::Failed(
                                                "action plugin returned an error".into(),
                                            ),
                                            handle.start_time.elapsed().as_millis() as u64,
                                        );
                                    }
                                }
                                action_error = Some(format!("Action error: {}", e));
                            }
                        }
                        if let Some(evidence_id) = evidence_id {
                            let dispatch = handle.patrol_state.lock().ok().and_then(|state| {
                                state.actions.last().map(|action| action.dispatch.clone())
                            });
                            log::info!(
                            "[patrol] action node='{}' id={} repetition={} index={} dispatch={:?}",
                            current_node.id,
                            evidence_id,
                            repetition,
                            action_index,
                            dispatch
                        );
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
                if let Some(attempt_id) = action_attempt_id {
                    if let Ok(mut state) = handle.patrol_state.lock() {
                        state.finish_attempt(
                            attempt_id,
                            handle.start_time.elapsed().as_millis() as u64,
                        );
                    }
                }

                if actions_fired > 0
                    && !matches!(current_node.post_wait_freezes.settings(), Ok(None))
                {
                    let phase_start = Instant::now();
                    match freeze::wait(
                        &stream_service,
                        &mut rx,
                        &current_node.post_wait_freezes,
                        expected_session,
                        &expected_stream_target,
                        None,
                        &handle,
                        None,
                    )
                    .await
                    {
                        FreezeOutcome::Stopped => {
                            was_stopped = true;
                            break 'pipeline;
                        }
                        FreezeOutcome::Failed(error) => {
                            log::warn!(
                                "[pipeline] node='{}' post-action stability failed: {error}",
                                current_node.id
                            );
                            if action_error.is_none() {
                                action_error =
                                    Some(format!("Post-action screen stability: {error}"));
                            } else {
                                log::warn!(
                                "[pipeline] node='{}' post-action stability also failed: {error}",
                                current_node.id
                            );
                            }
                        }
                        FreezeOutcome::Stable(frame) => {
                            log::info!(
                            "[pipeline] node='{}' post-action screen stable after {}ms (frame={})",
                            current_node.id,
                            phase_start.elapsed().as_millis(),
                            frame.sequence
                        );
                            last_frame_sequence = Some(frame.sequence);
                            if let Some(worker) = patrol_worker.as_ref() {
                                if let Ok(image) = image::load_from_memory(&frame.bytes) {
                                    worker.observe(PatrolSnapshot {
                                        node_id: current_node.id.clone(),
                                        screenshot: Arc::new(image),
                                        anchor_confirmed: false,
                                        capture_target: capture_target.clone(),
                                        capture_session: expected_session,
                                        frame_sequence: frame.sequence,
                                    });
                                }
                            }
                        }
                    }
                    if detailed_timings {
                        timings.insert(
                            "post_wait_freezes".into(),
                            phase_start.elapsed().as_millis() as u64,
                        );
                    }
                }

                let phase_start = std::time::Instant::now();
                if !cancellable_delay(&handle, Duration::from_millis(current_node.post_delay)).await
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

                if action_error.is_none() {
                    if let Some(patrol) = patrol.as_ref() {
                        let patrol = Arc::clone(patrol);
                        let node_id = current_node.id.clone();
                        let frame = screenshot.clone();
                        let chat = PatrolChatSink::new(
                            &app,
                            &handle.run_id,
                            &node_id,
                            &capture_target,
                            expected_session,
                            frame_sequence,
                        );
                        reference_tasks
                            .retain(|task: &tokio::task::JoinHandle<()>| !task.is_finished());
                        reference_tasks.push(tokio::task::spawn_blocking(move || {
                            match patrol.record(&node_id, &frame) {
                                Ok((path, bytes)) => chat.emit(
                                    "patrol",
                                    "reference_save",
                                    format!("Saved reference for {}", node_id),
                                    None,
                                    vec![
                                        ChatContent::Field {
                                            label: "Node".into(),
                                            value: node_id.clone(),
                                        },
                                        ChatContent::Field {
                                            label: "Saved at".into(),
                                            value: path,
                                        },
                                        ChatContent::Image {
                                            label: "Saved reference".into(),
                                            image_id: cache_image(bytes),
                                        },
                                    ],
                                ),
                                Err(error) => {
                                    log::warn!(
                                        "[patrol] reference recording failed node='{}': {}",
                                        node_id,
                                        error
                                    );
                                    chat.emit(
                                        "patrol",
                                        "error",
                                        format!("Could not save reference for {}", node_id),
                                        None,
                                        vec![ChatContent::Text {
                                            label: "Error".into(),
                                            text: error,
                                        }],
                                    );
                                }
                            }
                        }));
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

            // Stop the periodic clock before publishing terminal status. A failure
            // gets one final advisory validation against the latest captured frame.
            let (mut final_snapshot, final_action_frames) =
                if let Some(worker) = patrol_worker.take() {
                    worker.finish().await
                } else {
                    (None, Arc::new(std::sync::Mutex::new(VecDeque::new())))
                };
            if !was_stopped && (ctx.failed || !ctx.current_node_id.is_empty()) {
                // A new capture may have arrived while the failed node was running
                // its actions or delay. Prefer it over that node's earlier frame.
                if let (Some(snapshot), Some(active)) =
                    (final_snapshot.as_mut(), active_capture.as_ref())
                {
                    if snapshot.capture_session == active.session_id
                        && snapshot.capture_target == format!("{:?}", active.stream_target)
                    {
                        if let Some(frame) = stream_service.latest_frame_after(
                            active.session_id,
                            &active.stream_target,
                            Some(snapshot.frame_sequence),
                            None,
                        ) {
                            if let Ok(image) = image::load_from_memory(&frame.bytes) {
                                snapshot.screenshot = Arc::new(image);
                                snapshot.frame_sequence = frame.sequence;
                                snapshot.anchor_confirmed = false;
                                if let Ok(mut state) = handle.patrol_state.lock() {
                                    state.observe(ScreenObservation {
                                        expected_node_id: snapshot.node_id.clone(),
                                        target: snapshot.capture_target.clone(),
                                        session: snapshot.capture_session,
                                        frame: snapshot.frame_sequence,
                                        elapsed_ms: handle.start_time.elapsed().as_millis() as u64,
                                        recognizer_matched: None,
                                        recognizer_confidence: None,
                                    });
                                }
                            }
                        }
                    }
                }
                if let (Some(patrol), Some(snapshot)) = (patrol.as_ref(), final_snapshot.as_ref()) {
                    let mut final_sampler = PatrolSampler::new();
                    let action =
                        pending_action_for_snapshot(&final_action_frames, &handle, snapshot);
                    sample_patrol(
                        &mut final_sampler,
                        patrol,
                        &ctx.graph,
                        &handle,
                        &app,
                        snapshot,
                        action,
                        "failure",
                    );
                    final_sampler.complete_pending().await;
                } else if patrol.is_some() {
                    log::warn!("[patrol] final validation unavailable: no captured frame");
                }
            }

            if !was_stopped && ctx.failed {
                if let (Some(patrol), Some(snapshot)) = (patrol.as_ref(), final_snapshot.as_ref()) {
                    let failure_node = steps
                        .last()
                        .map(|step| step.node_id.clone())
                        .unwrap_or_else(|| snapshot.node_id.clone());
                    let recent_nodes = steps
                        .iter()
                        .map(|step| step.node_id.clone())
                        .collect::<Vec<_>>();
                    let failure_action = final_action_frames.lock().ok().and_then(|frames| {
                        frames
                            .iter()
                            .rev()
                            .find(|action| {
                                action.node_id == failure_node
                                    && action.target == snapshot.capture_target
                                    && action.session == snapshot.capture_session
                                    && action.before_frame < snapshot.frame_sequence
                            })
                            .cloned()
                    });
                    let chat = PatrolChatSink::new(
                        &app,
                        &handle.run_id,
                        &failure_node,
                        &snapshot.capture_target,
                        snapshot.capture_session,
                        snapshot.frame_sequence,
                    );
                    chat.emit(
                        "patrol",
                        "recovery_trigger",
                        "Assessing recovery before pipeline failure",
                        Some("recovery"),
                        vec![ChatContent::Field {
                            label: "Failure".into(),
                            value: ctx
                                .failure_error
                                .clone()
                                .unwrap_or_else(|| "Pipeline failed".into()),
                        }],
                    );
                    let input = RecoveryInput {
                        failure_node: failure_node.clone(),
                        failure_reason: ctx
                            .failure_error
                            .clone()
                            .unwrap_or_else(|| "Pipeline failed".into()),
                        actions: failure_action
                            .as_ref()
                            .map(|action| action.actions.clone())
                            .unwrap_or_default(),
                        before: failure_action.map(|action| action.before),
                        after: Arc::clone(&snapshot.screenshot),
                        recent_nodes,
                    };
                    let result = patrol.assess_recovery(&ctx.graph, &input, &chat).await;
                    let mut target_node = None;
                    let mut confidence = None;
                    let mut accepted = false;
                    let (decision, reason) = match result {
                        Ok(result) => {
                            target_node = result.target_node.clone();
                            confidence = Some(result.confidence);
                            let mut reason = result.reason;
                            let decision = match result.decision {
                                RecoveryDecision::Suggest => "suggest",
                                RecoveryDecision::Redirect => "redirect",
                                RecoveryDecision::NoRecovery => "no_recovery",
                                RecoveryDecision::Inconclusive => "inconclusive",
                            };
                            if patrol.config.recovery_policy == RecoveryPolicy::RedirectionOnly
                                && result.decision == RecoveryDecision::Redirect
                            {
                                if let Some(target) = result.target_node.as_deref() {
                                    let valid_node = ctx.graph.find_node(target).is_some_and(|node| {
                                        node.kind
                                            == crate::modules::pipeline::authoring::models::node::NodeKind::Normal
                                            && node.enabled
                                            && node.id != failure_node
                                    });
                                    let same_capture =
                                        active_capture.as_ref().is_some_and(|active| {
                                            active.session_id == snapshot.capture_session
                                                && format!("{:?}", active.stream_target)
                                                    == snapshot.capture_target
                                        });
                                    let route = (failure_node.clone(), target.to_string());
                                    let rejection = if result.confidence < patrol.config.threshold {
                                        Some(format!(
                                            "Redirect confidence {:.2} is below threshold {:.2}",
                                            result.confidence, patrol.config.threshold
                                        ))
                                    } else if !valid_node {
                                        Some(
                                            "Redirect target is not an eligible normal node".into(),
                                        )
                                    } else if !same_capture {
                                        Some("Capture target or session changed during recovery analysis".into())
                                    } else if redirect_count >= patrol.config.max_redirects {
                                        Some("Per-run Patrol redirect limit was reached".into())
                                    } else if redirect_routes.contains(&route) {
                                        Some(
                                            "The same failure-to-target redirect already ran"
                                                .into(),
                                        )
                                    } else if stop_requested(&handle) {
                                        Some("The run was stopped before redirection".into())
                                    } else {
                                        None
                                    };
                                    if let Some(rejection) = rejection {
                                        reason = rejection;
                                    } else {
                                        accepted = true;
                                        redirect_count += 1;
                                        redirect_routes.insert(route);
                                    }
                                }
                            }
                            (decision.to_string(), reason)
                        }
                        Err(error) => ("inconclusive".into(), error),
                    };
                    let report = PatrolRecoveryReport {
                        run_id: handle.run_id.clone(),
                        policy: match patrol.config.recovery_policy {
                            RecoveryPolicy::HintOnly => "hint_only",
                            RecoveryPolicy::RedirectionOnly => "redirection_only",
                        }
                        .into(),
                        failure_node: failure_node.clone(),
                        target_node: target_node.clone(),
                        decision,
                        confidence,
                        reason: reason.clone(),
                        accepted,
                        redirect_count,
                        max_redirects: patrol.config.max_redirects,
                    };
                    if let Ok(mut state) = handle.patrol_state.lock() {
                        state.record_recovery(RecoveryEvidence {
                            policy: report.policy.clone(),
                            failure_node: report.failure_node.clone(),
                            target_node: report.target_node.clone(),
                            decision: report.decision.clone(),
                            confidence: report.confidence,
                            reason: report.reason.clone(),
                            accepted: report.accepted,
                            redirect_count: report.redirect_count,
                        });
                    }
                    log::info!(
                    "[patrol] recovery run='{}' policy='{}' failure='{}' target={:?} accepted={} reason={:?}",
                    report.run_id,
                    report.policy,
                    report.failure_node,
                    report.target_node,
                    report.accepted,
                    report.reason
                );
                    chat.emit(
                        "patrol",
                        "recovery_report",
                        if accepted {
                            format!(
                                "Redirect accepted: {} to {}",
                                failure_node,
                                target_node.as_deref().unwrap_or("unknown")
                            )
                        } else if patrol.config.recovery_policy == RecoveryPolicy::HintOnly {
                            "Recovery hint prepared".into()
                        } else {
                            "Recovery redirect was not accepted".into()
                        },
                        Some("recovery"),
                        vec![
                            ChatContent::Field {
                                label: "Target".into(),
                                value: target_node.clone().unwrap_or_else(|| "None".into()),
                            },
                            ChatContent::Field {
                                label: "Reason".into(),
                                value: reason,
                            },
                        ],
                    );
                    let _ = app.emit("patrol-recovery", report);
                    if accepted {
                        let target = target_node.expect("accepted recovery has a target");
                        routing::redirect(&mut ctx, &handle, &target);
                        attempt_node = None;
                        solo_node.clear();
                        next_frame_not_before = Some(Instant::now());
                        patrol_worker = Some(PatrolWorker::start(
                            Arc::clone(patrol),
                            ctx.graph.clone(),
                            Arc::clone(&handle),
                            app.clone(),
                        ));
                        publish_status(&app, &handle, None);
                        continue 'execution;
                    }
                }
            }
            break 'execution;
        }
        for task in reference_tasks {
            let _ = task.await;
        }

        // Close the report channel before publishing a terminal state. This
        // also guards against any task that outlives the normal worker join.
        handle.close_patrol_reports();

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
    patrol: Option<PatrolRuntime>,
    app: AppHandle,
) -> EngineResult {
    PipelineRunSession::new(
        plan,
        registry,
        handle,
        stream_service,
        user_params,
        save_debug_images,
        detailed_timings,
        pipeline_startup_ms,
        patrol,
        app,
    )
    .run()
    .await
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

#[cfg(test)]
mod parameter_tests {
    use super::patrol_worker::{next_patrol_due, PatrolInFlight};
    use super::*;
    use crate::modules::capture::services::stream::StreamTarget;
    use crate::modules::pipeline::authoring::models::action::Action;
    use crate::modules::pipeline::runtime::engine::patrol_state::EffectOutcome;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn patrol_sampling_is_single_flight() {
        let mut sampler = PatrolSampler::new();
        let in_flight = sampler.try_sample().expect("first cycle is sampled");
        assert!(sampler.try_sample().is_none());
        in_flight.store(false, Ordering::Release);
        assert!(sampler.try_sample().is_some());
    }

    #[test]
    fn patrol_cycles_wait_for_both_period_and_response() {
        let started = tokio::time::Instant::now();
        let period = Duration::from_secs(20);
        assert_eq!(
            next_patrol_due(started, started + Duration::from_secs(8), period),
            started + period
        );
        assert_eq!(
            next_patrol_due(started, started + Duration::from_secs(27), period),
            started + Duration::from_secs(27)
        );
    }

    #[test]
    fn patrol_prioritizes_a_sent_unresolved_action_with_a_newer_same_session_frame() {
        let handle = RunHandle::new("run".into());
        let attempt_id = {
            let mut state = handle.patrol_state.lock().unwrap();
            let attempt_id = state.begin_attempt(
                "Click",
                vec!["Result".into()],
                None,
                "Screen { index: 0 }",
                3,
                10,
                0,
            );
            let action_id = state.begin_action(
                attempt_id,
                "Click",
                0,
                0,
                &Action::DoNothing,
                "Screen { index: 0 }",
                3,
                10,
                0,
            );
            state.finish_action(action_id, DispatchOutcome::Sent, 1);
            state.finish_attempt(attempt_id, 1);
            attempt_id
        };
        let frames = Arc::new(std::sync::Mutex::new(VecDeque::from([
            PatrolActionFrames {
                attempt_id,
                node_id: "Click".into(),
                actions: vec!["DoNothing".into()],
                expected_successors: vec!["Result".into()],
                error_successor: None,
                before: Arc::new(image::DynamicImage::new_rgb8(2, 2)),
                target: "Screen { index: 0 }".into(),
                session: 3,
                before_frame: 10,
            },
        ])));
        let mut snapshot = PatrolSnapshot {
            node_id: "Result".into(),
            screenshot: Arc::new(image::DynamicImage::new_rgb8(2, 2)),
            anchor_confirmed: false,
            capture_target: "Screen { index: 0 }".into(),
            capture_session: 3,
            frame_sequence: 10,
        };
        assert!(pending_action_for_snapshot(&frames, &handle, &snapshot).is_none());
        snapshot.frame_sequence = 11;
        assert_eq!(
            pending_action_for_snapshot(&frames, &handle, &snapshot)
                .map(|attempt| attempt.attempt_id),
            Some(attempt_id)
        );
        handle
            .patrol_state
            .lock()
            .unwrap()
            .apply_action_effect(
                attempt_id,
                "Screen { index: 0 }",
                3,
                11,
                EffectOutcome::InProgress {
                    session: 3,
                    frame: 11,
                    confidence: 0.8,
                    reason: "loading".into(),
                },
            )
            .unwrap();
        assert!(pending_action_for_snapshot(&frames, &handle, &snapshot).is_none());
        snapshot.frame_sequence = 12;
        assert!(pending_action_for_snapshot(&frames, &handle, &snapshot).is_some());
        snapshot.capture_session = 4;
        assert!(pending_action_for_snapshot(&frames, &handle, &snapshot).is_none());
    }

    #[tokio::test]
    async fn terminal_run_cancels_unfinished_patrol_work() {
        let mut sampler = PatrolSampler::new();
        let in_flight = sampler.try_sample().unwrap();
        let emitted = Arc::new(AtomicBool::new(false));
        let emitted_in_task = Arc::clone(&emitted);
        sampler.task = Some(tokio::spawn(async move {
            let _guard = PatrolInFlight(in_flight);
            tokio::time::sleep(Duration::from_secs(30)).await;
            emitted_in_task.store(true, Ordering::Release);
        }));
        sampler.cancel_pending().await;
        assert!(!emitted.load(Ordering::Acquire));
        assert!(!sampler.in_flight.load(Ordering::Acquire));
        assert!(sampler.task.is_none());
    }

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

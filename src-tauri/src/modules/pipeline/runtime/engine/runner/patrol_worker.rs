use super::super::RunHandle;
use crate::modules::capture::services::stream::StreamService;
use crate::modules::pipeline::runtime::chat::{ChatContent, PatrolChatKind, PatrolChatSink};
use crate::modules::pipeline::runtime::engine::patrol_state::{
    ActionAttemptEvidence, DispatchOutcome, EffectOutcome, PatrolDecisionEvidence,
};
use crate::modules::pipeline::runtime::patrol::{
    ActionEffectKind, ActionVerificationInput, LocatedState, LocationSource, PatrolRuntime,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub(super) struct PatrolSampler {
    pub(super) in_flight: Arc<AtomicBool>,
    pub(super) task: Option<tokio::task::JoinHandle<()>>,
}

impl PatrolSampler {
    pub(super) fn new() -> Self {
        Self {
            in_flight: Arc::new(AtomicBool::new(false)),
            task: None,
        }
    }

    pub(super) fn try_sample(&mut self) -> Option<Arc<AtomicBool>> {
        if self
            .in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return None;
        }
        Some(Arc::clone(&self.in_flight))
    }

    pub(super) async fn complete_pending(&mut self) {
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.in_flight.store(false, Ordering::Release);
    }

    pub(super) async fn cancel_pending(&mut self) {
        if let Some(task) = self.task.take() {
            if !task.is_finished() {
                log::info!("[patrol] cancelling unfinished VLM sample as pipeline ends");
                task.abort();
            }
            let _ = task.await;
        }
        self.in_flight.store(false, Ordering::Release);
    }
}

impl Drop for PatrolSampler {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub(super) struct PatrolInFlight(pub(super) Arc<AtomicBool>);

impl Drop for PatrolInFlight {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(Clone)]
pub(super) struct PatrolSnapshot {
    pub(super) node_id: String,
    pub(super) screenshot: Arc<image::DynamicImage>,
    pub(super) anchor_confirmed: bool,
    pub(super) capture_target: String,
    pub(super) capture_session: u64,
    pub(super) frame_sequence: u64,
    pub(super) stable: bool,
}

#[derive(Clone)]
pub(super) struct PatrolActionFrames {
    pub(super) attempt_id: u64,
    pub(super) node_id: String,
    pub(super) actions: Vec<String>,
    pub(super) expected_successors: Vec<String>,
    pub(super) error_successor: Option<String>,
    pub(super) before: Arc<image::DynamicImage>,
    pub(super) target: String,
    pub(super) session: u64,
    pub(super) before_frame: u64,
    pub(super) preferred_after: Option<PatrolSnapshot>,
    pub(super) preferred_from_post_freeze: bool,
}

pub(super) struct PatrolWorker {
    latest: Arc<std::sync::Mutex<Option<PatrolSnapshot>>>,
    action_frames: Arc<std::sync::Mutex<VecDeque<PatrolActionFrames>>>,
    stream_service: Arc<StreamService>,
    handle: Arc<RunHandle>,
    collectors: Arc<std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    app: AppHandle,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}

pub(super) fn next_patrol_due(
    started: tokio::time::Instant,
    completed: tokio::time::Instant,
    period: Duration,
) -> tokio::time::Instant {
    (started + period).max(completed)
}

impl PatrolWorker {
    pub(super) fn start(
        patrol: Arc<PatrolRuntime>,
        graph: crate::modules::pipeline::authoring::models::graph::PipelineGraph,
        handle: Arc<RunHandle>,
        app: AppHandle,
        stream_service: Arc<StreamService>,
    ) -> Self {
        let latest = Arc::new(std::sync::Mutex::new(None::<PatrolSnapshot>));
        let observations = Arc::clone(&latest);
        let action_frames = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let worker_action_frames = Arc::clone(&action_frames);
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let collectors = Arc::new(std::sync::Mutex::new(Vec::new()));
        let collector_handle = Arc::clone(&handle);
        let collector_app = app.clone();
        let period = Duration::from_secs(patrol.config.cycle_seconds);
        let task = tokio::spawn(async move {
            let mut sampler = PatrolSampler::new();
            let mut due = tokio::time::Instant::now() + period;
            loop {
                tokio::select! {
                    _ = &mut stopped => break,
                    _ = tokio::time::sleep_until(due) => {
                        let started = tokio::time::Instant::now();
                        let snapshot = observations.lock().ok().and_then(|value| value.clone());
                        if let Some(snapshot) = snapshot {
                            if !patrol.periodic_allowed() {
                                if patrol.take_circuit_report() {
                                    let reason = "Patrol VLM circuit opened after three consecutive transport failures; periodic requests are paused for this run".to_string();
                                    let chat = PatrolChatSink::new(
                                        &app,
                                        &handle.run_id,
                                        &snapshot.node_id,
                                        &snapshot.capture_target,
                                        snapshot.capture_session,
                                        snapshot.frame_sequence,
                                    );
                                    chat.emit(
                                        "patrol",
                                        PatrolChatKind::Error,
                                        "Patrol endpoint circuit is open",
                                        None,
                                        vec![ChatContent::Text {
                                            label: "Diagnostic".into(),
                                            text: reason.clone(),
                                        }],
                                    );
                                    let _ = app.emit("patrol-report", PatrolReport {
                                        run_id: handle.run_id.clone(),
                                        trigger: "cycle".into(),
                                        task: "state_match".into(),
                                        action_attempt_id: None,
                                        expected_node_id: snapshot.node_id.clone(),
                                        matched_node_id: None,
                                        target: snapshot.capture_target.clone(),
                                        session: snapshot.capture_session,
                                        frame: snapshot.frame_sequence,
                                        outcome: "circuit_open".into(),
                                        reason: Some(reason),
                                    });
                                }
                                due = next_patrol_due(started, tokio::time::Instant::now(), period);
                                continue;
                            }
                            let action = pending_action_for_snapshot(
                                &worker_action_frames,
                                &handle,
                                &snapshot,
                            );
                            sample_patrol(&mut sampler, &patrol, &graph, &handle, &app, &snapshot, action, "cycle");
                            // The next request starts when both this response
                            // and the minimum cycle interval are complete.
                            tokio::select! {
                                _ = &mut stopped => break,
                                _ = sampler.complete_pending() => {}
                            }
                        } else {
                            log::debug!("[patrol] cycle elapsed without a captured frame");
                        }
                        due = next_patrol_due(started, tokio::time::Instant::now(), period);
                    }
                }
            }
            sampler.cancel_pending().await;
        });
        Self {
            latest,
            action_frames,
            stream_service,
            handle: collector_handle,
            collectors,
            app: collector_app,
            stop: Some(stop),
            task: Some(task),
        }
    }

    pub(super) fn observe(&self, snapshot: PatrolSnapshot) {
        if snapshot.stable {
            if let Ok(mut attempts) = self.action_frames.lock() {
                for attempt in attempts.iter_mut().rev() {
                    if attempt.target == snapshot.capture_target
                        && attempt.session == snapshot.capture_session
                        && attempt.before_frame < snapshot.frame_sequence
                    {
                        attempt.preferred_after = Some(snapshot.clone());
                        attempt.preferred_from_post_freeze = true;
                    }
                }
            }
        }
        if let Ok(mut latest) = self.latest.lock() {
            *latest = Some(snapshot);
        }
    }

    pub(super) fn set_anchor_confirmed(&self, frame_sequence: u64, confirmed: bool) {
        if let Ok(mut latest) = self.latest.lock() {
            if let Some(snapshot) = latest.as_mut() {
                if snapshot.frame_sequence == frame_sequence {
                    snapshot.anchor_confirmed = confirmed;
                }
            }
        }
    }

    pub(super) fn record_action_attempt(&self, attempt: PatrolActionFrames) {
        let attempt_id = attempt.attempt_id;
        let node_id = attempt.node_id.clone();
        let target = attempt.target.clone();
        let session = attempt.session;
        let before_frame = attempt.before_frame;
        if let Ok(mut attempts) = self.action_frames.lock() {
            attempts.push_back(attempt);
            while attempts.len() > 32 {
                attempts.pop_front();
            }
        }
        let frames = Arc::clone(&self.action_frames);
        let handle = Arc::clone(&self.handle);
        let app = self.app.clone();
        let mut rx = self.stream_service.subscribe();
        let collector = tokio::spawn(async move {
            let mut active_elapsed = Duration::ZERO;
            let mut last_tick = tokio::time::Instant::now();
            let mut last_sample_at: Option<tokio::time::Instant> = None;
            let mut previous: Option<Arc<image::DynamicImage>> = None;
            let mut stable_samples = 0u8;
            let mut latest: Option<PatrolSnapshot> = None;
            while active_elapsed < Duration::from_secs(2) {
                if super::stop_requested(&handle) {
                    return;
                }
                let paused = matches!(handle.get_state(), super::PipelineState::Paused);
                let received = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
                let now = tokio::time::Instant::now();
                if !paused {
                    active_elapsed += now.saturating_duration_since(last_tick);
                }
                last_tick = now;
                let Ok(Ok(frame)) = received else { continue };
                if paused
                    || frame.session_id != session
                    || format!("{:?}", frame.target) != target
                    || frame.sequence <= before_frame
                    || last_sample_at
                        .is_some_and(|at| now.duration_since(at) < Duration::from_millis(200))
                {
                    continue;
                }
                let Ok(image) = image::load_from_memory(&frame.bytes) else {
                    continue;
                };
                let image = Arc::new(image);
                stable_samples = if previous
                    .as_ref()
                    .is_some_and(|previous| patrol_frame_similarity(previous, &image) >= 0.98)
                {
                    stable_samples.saturating_add(1)
                } else {
                    1
                };
                previous = Some(Arc::clone(&image));
                last_sample_at = Some(now);
                latest = Some(PatrolSnapshot {
                    node_id: String::new(),
                    screenshot: image,
                    anchor_confirmed: false,
                    capture_target: target.clone(),
                    capture_session: session,
                    frame_sequence: frame.sequence,
                    stable: stable_samples >= 3,
                });
                if stable_samples >= 3 {
                    break;
                }
            }
            if let Some(snapshot) = latest {
                if let Ok(mut attempts) = frames.lock() {
                    if let Some(attempt) = attempts
                        .iter_mut()
                        .find(|attempt| attempt.attempt_id == attempt_id)
                    {
                        if !attempt.preferred_from_post_freeze
                            && attempt
                                .preferred_after
                                .as_ref()
                                .is_none_or(|existing| !existing.stable || snapshot.stable)
                        {
                            attempt.preferred_after = Some(snapshot);
                        }
                    }
                }
            } else {
                let reason = format!(
                    "Action attempt {attempt_id} has no newer frame on the same target/session; effect verification skipped"
                );
                log::warn!("[patrol] {reason}");
                PatrolChatSink::new(
                    &app,
                    &handle.run_id,
                    &node_id,
                    &target,
                    session,
                    before_frame,
                )
                .emit(
                    "patrol",
                    PatrolChatKind::Error,
                    "Action verification skipped",
                    Some("action"),
                    vec![ChatContent::Text {
                        label: "Diagnostic".into(),
                        text: reason,
                    }],
                );
            }
        });
        if let Ok(mut collectors) = self.collectors.lock() {
            collectors.retain(|task| !task.is_finished());
            collectors.push(collector);
        }
    }

    pub(super) async fn finish(
        mut self,
    ) -> (
        Option<PatrolSnapshot>,
        Arc<std::sync::Mutex<VecDeque<PatrolActionFrames>>>,
    ) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        let collectors = self
            .collectors
            .lock()
            .map(|mut collectors| std::mem::take(&mut *collectors))
            .unwrap_or_default();
        for collector in collectors {
            let _ = collector.await;
        }
        let latest = self.latest.lock().ok().and_then(|value| value.clone());
        (latest, Arc::clone(&self.action_frames))
    }
}

pub(super) fn pending_action_for_snapshot(
    frames: &Arc<std::sync::Mutex<VecDeque<PatrolActionFrames>>>,
    handle: &RunHandle,
    snapshot: &PatrolSnapshot,
) -> Option<PatrolActionFrames> {
    let state = handle.patrol_state.lock().ok()?;
    let frames = frames.lock().ok()?;
    frames
        .iter()
        .rev()
        .find(|frames| {
            if (frames.expected_successors.is_empty() && frames.error_successor.is_none())
                || frames.target != snapshot.capture_target
                || frames.session != snapshot.capture_session
                || frames.before_frame >= snapshot.frame_sequence
            {
                return false;
            }
            state
                .action_attempts
                .iter()
                .find(|attempt| attempt.id == frames.attempt_id)
                .is_some_and(|attempt| {
                    attempt.completed_ms.is_some()
                        && snapshot.frame_sequence
                            > attempt.after_frame.unwrap_or(attempt.before_frame)
                        && attempt.action_ids.iter().any(|id| {
                            state.actions.iter().any(|action| {
                                action.id == *id && action.dispatch == DispatchOutcome::Sent
                            })
                        })
                        && matches!(
                            attempt.effect,
                            EffectOutcome::Unverified
                                | EffectOutcome::InProgress { .. }
                                | EffectOutcome::Inconclusive { .. }
                        )
                })
        })
        .cloned()
}

impl Drop for PatrolWorker {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
        if let Ok(mut collectors) = self.collectors.lock() {
            for collector in collectors.drain(..) {
                collector.abort();
            }
        }
    }
}

fn patrol_frame_similarity(left: &image::DynamicImage, right: &image::DynamicImage) -> f64 {
    use image::imageops::FilterType;
    let left = left.resize_exact(64, 64, FilterType::Triangle).to_luma8();
    let right = right.resize_exact(64, 64, FilterType::Triangle).to_luma8();
    let difference = left
        .pixels()
        .zip(right.pixels())
        .map(|(left, right)| (f64::from(left[0]) - f64::from(right[0])).abs())
        .sum::<f64>();
    1.0 - difference / (64.0 * 64.0 * 255.0)
}

#[cfg(test)]
mod stability_tests {
    use super::*;

    fn attempt(effect: EffectOutcome, stable: Option<bool>) -> ActionAttemptEvidence {
        ActionAttemptEvidence {
            id: 1,
            node_id: "ClickLogin".into(),
            action_ids: vec![],
            expected_successors: vec!["Dashboard".into()],
            error_successor: Some("LoginError".into()),
            target: "Window(7)".into(),
            source_session: 3,
            before_frame: 10,
            after_frame: Some(11),
            after_frame_stable: stable,
            started_ms: 1,
            completed_ms: Some(2),
            effect,
        }
    }

    #[test]
    fn patrol_frame_similarity_distinguishes_stable_and_changed_frames() {
        let black = image::DynamicImage::new_luma8(8, 8);
        let mut white = image::GrayImage::new(8, 8);
        white.fill(255);
        let white = image::DynamicImage::ImageLuma8(white);
        assert_eq!(patrol_frame_similarity(&black, &black), 1.0);
        assert!(patrol_frame_similarity(&black, &white) < 0.01);
    }

    #[test]
    fn only_stable_declared_successor_effects_establish_location() {
        let achieved = attempt(
            EffectOutcome::Achieved {
                matched_successor: "Dashboard".into(),
                session: 3,
                frame: 11,
                confidence: 0.91,
                reason: "visible".into(),
            },
            Some(true),
        );
        let location = located_state_from_attempt(&achieved).unwrap();
        assert_eq!(location.node_id, "Dashboard");
        assert_eq!(location.source, LocationSource::ActionExpectedSuccessor);
        assert_eq!(location.confidence, 0.91);

        let mut unstable = achieved.clone();
        unstable.after_frame_stable = Some(false);
        assert!(located_state_from_attempt(&unstable).is_none());
        let unchanged = attempt(
            EffectOutcome::Unchanged {
                session: 3,
                frame: 11,
                confidence: 0.99,
                reason: "same".into(),
            },
            Some(true),
        );
        assert!(located_state_from_attempt(&unchanged).is_none());
    }
}

#[derive(Clone, serde::Serialize)]
struct PatrolReport {
    run_id: String,
    trigger: String,
    task: String,
    action_attempt_id: Option<u64>,
    expected_node_id: String,
    matched_node_id: Option<String>,
    target: String,
    session: u64,
    frame: u64,
    outcome: String,
    reason: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub(super) struct PatrolRecoveryReport {
    pub(super) run_id: String,
    pub(super) policy: String,
    pub(super) failure_node: String,
    pub(super) target_node: Option<String>,
    pub(super) decision: String,
    pub(super) confidence: Option<f64>,
    pub(super) reason: String,
    pub(super) accepted: bool,
    pub(super) redirect_count: u32,
    pub(super) max_redirects: u32,
    pub(super) location_node: Option<String>,
    pub(super) location_confidence: Option<f64>,
    pub(super) location_source: Option<String>,
    pub(super) location_frame_stable: Option<bool>,
    pub(super) location_uncertain: bool,
}

pub(super) fn located_state_from_action(
    handle: &RunHandle,
    attempt_id: u64,
) -> Option<LocatedState> {
    let state = handle.patrol_state.lock().ok()?;
    let attempt = state
        .action_attempts
        .iter()
        .find(|attempt| attempt.id == attempt_id)?;
    located_state_from_attempt(attempt)
}

fn located_state_from_attempt(attempt: &ActionAttemptEvidence) -> Option<LocatedState> {
    if attempt.after_frame_stable != Some(true) {
        return None;
    }
    match &attempt.effect {
        EffectOutcome::Achieved {
            matched_successor,
            session,
            frame,
            confidence,
            reason,
        } => Some(LocatedState {
            node_id: matched_successor.clone(),
            confidence: *confidence,
            source: LocationSource::ActionExpectedSuccessor,
            capture_target: attempt.target.clone(),
            capture_session: *session,
            frame_sequence: *frame,
            frame_stable: true,
            reason: reason.clone(),
        }),
        EffectOutcome::Unexpected {
            matched_node: Some(matched_node),
            session,
            frame,
            confidence,
            reason,
        } => Some(LocatedState {
            node_id: matched_node.clone(),
            confidence: *confidence,
            source: LocationSource::ActionErrorSuccessor,
            capture_target: attempt.target.clone(),
            capture_session: *session,
            frame_sequence: *frame,
            frame_stable: true,
            reason: reason.clone(),
        }),
        _ => None,
    }
}

pub(super) async fn locate_state_for_failure(
    patrol: &PatrolRuntime,
    graph: &crate::modules::pipeline::authoring::models::graph::PipelineGraph,
    handle: &RunHandle,
    app: &AppHandle,
    snapshot: &PatrolSnapshot,
) -> Result<LocatedState, String> {
    let recent_nodes = handle
        .step_results
        .lock()
        .map(|steps| {
            steps
                .iter()
                .map(|step| step.node_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let chat = PatrolChatSink::new(
        app,
        &handle.run_id,
        &snapshot.node_id,
        &snapshot.capture_target,
        snapshot.capture_session,
        snapshot.frame_sequence,
    );
    let result = patrol
        .verify(
            graph,
            &snapshot.node_id,
            &snapshot.screenshot,
            snapshot.anchor_confirmed,
            &recent_nodes,
            &chat,
        )
        .await;
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            let _ = app.emit(
                "patrol-report",
                PatrolReport {
                    run_id: handle.run_id.clone(),
                    trigger: "failure".into(),
                    task: "state_match".into(),
                    action_attempt_id: None,
                    expected_node_id: snapshot.node_id.clone(),
                    matched_node_id: None,
                    target: snapshot.capture_target.clone(),
                    session: snapshot.capture_session,
                    frame: snapshot.frame_sequence,
                    outcome: "inconclusive".into(),
                    reason: Some(error.clone()),
                },
            );
            return Err(error);
        }
    };
    let outcome = if result.node_id == snapshot.node_id {
        "aligned"
    } else {
        "possible_mismatch"
    };
    let location = LocatedState {
        node_id: result.node_id.clone(),
        confidence: result.confidence,
        source: LocationSource::StateMatcher,
        capture_target: snapshot.capture_target.clone(),
        capture_session: snapshot.capture_session,
        frame_sequence: snapshot.frame_sequence,
        frame_stable: snapshot.stable,
        reason: result.reason.clone(),
    };
    if let Ok(mut state) = handle.patrol_state.lock() {
        state.decide(PatrolDecisionEvidence {
            expected_node_id: snapshot.node_id.clone(),
            matched_node_id: Some(result.node_id.clone()),
            target: snapshot.capture_target.clone(),
            session: snapshot.capture_session,
            frame: snapshot.frame_sequence,
            elapsed_ms: handle.start_time.elapsed().as_millis() as u64,
            outcome: outcome.into(),
            reason: Some(result.reason.clone()),
        });
    }
    chat.emit(
        "patrol",
        PatrolChatKind::LocationHandoff,
        format!("Validated current location at {}", location.node_id),
        Some("recovery"),
        vec![
            ChatContent::Field {
                label: "Node".into(),
                value: location.node_id.clone(),
            },
            ChatContent::Field {
                label: "Confidence".into(),
                value: format!("{:.2}", location.confidence),
            },
            ChatContent::Field {
                label: "Source".into(),
                value: location.source.as_str().into(),
            },
            ChatContent::Field {
                label: "Capture".into(),
                value: format!(
                    "{} · session {} · frame {} · {}",
                    location.capture_target,
                    location.capture_session,
                    location.frame_sequence,
                    if location.frame_stable {
                        "stable"
                    } else {
                        "unstable"
                    }
                ),
            },
        ],
    );
    let _ = app.emit(
        "patrol-report",
        PatrolReport {
            run_id: handle.run_id.clone(),
            trigger: "failure".into(),
            task: "state_match".into(),
            action_attempt_id: None,
            expected_node_id: snapshot.node_id.clone(),
            matched_node_id: Some(result.node_id),
            target: snapshot.capture_target.clone(),
            session: snapshot.capture_session,
            frame: snapshot.frame_sequence,
            outcome: outcome.into(),
            reason: Some(result.reason),
        },
    );
    Ok(location)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sample_patrol(
    sampler: &mut PatrolSampler,
    patrol: &Arc<PatrolRuntime>,
    graph: &crate::modules::pipeline::authoring::models::graph::PipelineGraph,
    handle: &Arc<RunHandle>,
    app: &AppHandle,
    snapshot: &PatrolSnapshot,
    action: Option<PatrolActionFrames>,
    trigger: &str,
) {
    let Some(in_flight) = sampler.try_sample() else {
        return;
    };
    let _guard = PatrolInFlight(in_flight);
    let patrol = Arc::clone(patrol);
    let graph = graph.clone();
    let handle = Arc::clone(handle);
    let app = app.clone();
    let node_id = snapshot.node_id.clone();
    let action_snapshot = action
        .as_ref()
        .and_then(|action| action.preferred_after.clone())
        .unwrap_or_else(|| snapshot.clone());
    let screenshot = action_snapshot.screenshot.clone();
    let anchor_confirmed = snapshot.anchor_confirmed;
    let capture_target = action_snapshot.capture_target.clone();
    let capture_session = action_snapshot.capture_session;
    let frame_sequence = action_snapshot.frame_sequence;
    let after_frame_stable = action_snapshot.stable;
    let trigger = trigger.to_string();
    let task = tokio::spawn(async move {
        let _guard = _guard;
        let chat = PatrolChatSink::new(
            &app,
            &handle.run_id,
            &node_id,
            &capture_target,
            capture_session,
            frame_sequence,
        );
        chat.emit(
            "patrol",
            PatrolChatKind::Trigger,
            if trigger == "failure" {
                "Validating before pipeline failure"
            } else {
                "Scheduled patrol validation"
            },
            None,
            vec![ChatContent::Field {
                label: "Trigger".into(),
                value: trigger.clone(),
            }],
        );
        if let Some(action) = action {
            let input = ActionVerificationInput {
                attempt_id: action.attempt_id,
                node_id: action.node_id.clone(),
                actions: action.actions.clone(),
                expected_successors: action.expected_successors.clone(),
                error_successor: action.error_successor.clone(),
                before: action.before,
                after: Arc::clone(&screenshot),
            };
            let result = patrol.verify_action_effect(&graph, &input, &chat).await;
            if !handle.patrol_reports_open() {
                log::debug!(
                    "[patrol] discarding action report for terminal run '{}'",
                    handle.run_id
                );
                return;
            }
            let (matched_node_id, outcome, confidence, reason, effect) = match result {
                Ok(result) => {
                    let outcome = match result.effect {
                        ActionEffectKind::Achieved => "achieved",
                        ActionEffectKind::Unchanged => "unchanged",
                        ActionEffectKind::InProgress => "in_progress",
                        ActionEffectKind::Unexpected => "unexpected",
                        ActionEffectKind::Inconclusive => "inconclusive",
                    };
                    let effect = match result.effect {
                        ActionEffectKind::Achieved => EffectOutcome::Achieved {
                            matched_successor: result.matched_node.clone().unwrap_or_default(),
                            session: capture_session,
                            frame: frame_sequence,
                            confidence: result.confidence,
                            reason: result.reason.clone(),
                        },
                        ActionEffectKind::Unchanged => EffectOutcome::Unchanged {
                            session: capture_session,
                            frame: frame_sequence,
                            confidence: result.confidence,
                            reason: result.reason.clone(),
                        },
                        ActionEffectKind::InProgress => EffectOutcome::InProgress {
                            session: capture_session,
                            frame: frame_sequence,
                            confidence: result.confidence,
                            reason: result.reason.clone(),
                        },
                        ActionEffectKind::Unexpected => EffectOutcome::Unexpected {
                            matched_node: result.matched_node.clone(),
                            session: capture_session,
                            frame: frame_sequence,
                            confidence: result.confidence,
                            reason: result.reason.clone(),
                        },
                        ActionEffectKind::Inconclusive => EffectOutcome::Inconclusive {
                            session: capture_session,
                            frame: frame_sequence,
                            reason: result.reason.clone(),
                        },
                    };
                    (
                        result.matched_node,
                        outcome,
                        Some(result.confidence),
                        Some(result.reason),
                        effect,
                    )
                }
                Err(error) => {
                    let (outcome, error) = if patrol.take_circuit_report() {
                        (
                            "circuit_open",
                            format!(
                                "{error}. Patrol VLM circuit opened after three consecutive transport failures; periodic requests are paused for this run"
                            ),
                        )
                    } else {
                        ("inconclusive", error)
                    };
                    (
                        None,
                        outcome,
                        None,
                        Some(error.clone()),
                        EffectOutcome::Inconclusive {
                            session: capture_session,
                            frame: frame_sequence,
                            reason: error,
                        },
                    )
                }
            };
            if let Ok(mut state) = handle.patrol_state.lock() {
                if let Err(error) = state.apply_action_effect(
                    action.attempt_id,
                    &capture_target,
                    capture_session,
                    frame_sequence,
                    effect,
                ) {
                    log::warn!("[patrol] rejected action-effect evidence: {error}");
                } else {
                    state.set_after_frame_stability(action.attempt_id, after_frame_stable);
                }
            }
            let report = PatrolReport {
                run_id: handle.run_id.clone(),
                trigger: trigger.clone(),
                task: "action_effect".into(),
                action_attempt_id: Some(action.attempt_id),
                expected_node_id: action.node_id,
                matched_node_id,
                target: capture_target,
                session: capture_session,
                frame: frame_sequence,
                outcome: outcome.into(),
                reason: reason.clone(),
            };
            log::info!(
                "[patrol] action report run='{}' trigger='{}' attempt={} frame={} outcome='{}' confidence={:?} reason={:?}",
                report.run_id, report.trigger, action.attempt_id, report.frame, report.outcome, confidence, report.reason
            );
            chat.emit(
                "patrol",
                PatrolChatKind::ActionReport,
                format!("Action effect: {}", outcome.replace('_', " ")),
                Some("action"),
                vec![
                    ChatContent::Field {
                        label: "Attempt".into(),
                        value: action.attempt_id.to_string(),
                    },
                    ChatContent::Field {
                        label: "Result".into(),
                        value: outcome.replace('_', " "),
                    },
                    ChatContent::Field {
                        label: "Confidence".into(),
                        value: confidence
                            .map(|value| format!("{value:.2}"))
                            .unwrap_or_else(|| "Unavailable".into()),
                    },
                    ChatContent::Field {
                        label: "Reason".into(),
                        value: reason.unwrap_or_else(|| "No reason supplied".into()),
                    },
                    ChatContent::Field {
                        label: "Before frame".into(),
                        value: format!("session {}, frame {}", action.session, action.before_frame),
                    },
                    ChatContent::Field {
                        label: "After frame".into(),
                        value: format!(
                            "session {capture_session}, frame {frame_sequence} ({})",
                            if after_frame_stable {
                                "stable"
                            } else {
                                "unstable"
                            }
                        ),
                    },
                ],
            );
            let _ = app.emit("patrol-report", report);
            return;
        }
        let recent_nodes = handle
            .step_results
            .lock()
            .map(|steps| {
                steps
                    .iter()
                    .map(|step| step.node_id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let result = patrol
            .verify(
                &graph,
                &node_id,
                &screenshot,
                anchor_confirmed,
                &recent_nodes,
                &chat,
            )
            .await;
        if !handle.patrol_reports_open() {
            log::debug!(
                "[patrol] discarding report for terminal run '{}'",
                handle.run_id
            );
            return;
        }
        let circuit_open = patrol.take_circuit_report();
        let (matched_node_id, outcome, reason) = match result {
            Ok(result) if result.node_id == node_id => {
                (Some(result.node_id), "aligned", Some(result.reason))
            }
            Ok(result) => (
                Some(result.node_id),
                "possible_mismatch",
                Some(result.reason),
            ),
            Err(error) if circuit_open => (
                None,
                "circuit_open",
                Some(format!(
                    "{error}. Patrol VLM circuit opened after three consecutive transport failures; periodic requests are paused for this run"
                )),
            ),
            Err(error) => (None, "inconclusive", Some(error)),
        };
        let report = PatrolReport {
            run_id: handle.run_id.clone(),
            trigger: trigger.clone(),
            task: "state_match".into(),
            action_attempt_id: None,
            expected_node_id: node_id.clone(),
            matched_node_id: matched_node_id.clone(),
            target: capture_target.clone(),
            session: capture_session,
            frame: frame_sequence,
            outcome: outcome.into(),
            reason: reason.clone(),
        };
        if let Ok(mut state) = handle.patrol_state.lock() {
            state.decide(PatrolDecisionEvidence {
                expected_node_id: node_id,
                matched_node_id,
                target: capture_target,
                session: capture_session,
                frame: frame_sequence,
                elapsed_ms: handle.start_time.elapsed().as_millis() as u64,
                outcome: outcome.into(),
                reason,
            });
        }
        log::info!(
            "[patrol] report run='{}' trigger='{}' frame={} expected='{}' matched={:?} outcome='{}' reason={:?}",
            report.run_id,
            report.trigger,
            report.frame,
            report.expected_node_id,
            report.matched_node_id,
            report.outcome,
            report.reason
        );
        chat.emit(
            "patrol",
            PatrolChatKind::Report,
            format!("Patrol report: {}", outcome.replace('_', " ")),
            None,
            vec![
                ChatContent::Field {
                    label: "Expected".into(),
                    value: report.expected_node_id.clone(),
                },
                ChatContent::Field {
                    label: "Possible match".into(),
                    value: report
                        .matched_node_id
                        .clone()
                        .unwrap_or_else(|| "None".into()),
                },
                ChatContent::Field {
                    label: "Reason".into(),
                    value: report
                        .reason
                        .clone()
                        .unwrap_or_else(|| "No issue reported".into()),
                },
            ],
        );
        let _ = app.emit("patrol-report", report);
    });
    sampler.task = Some(task);
}

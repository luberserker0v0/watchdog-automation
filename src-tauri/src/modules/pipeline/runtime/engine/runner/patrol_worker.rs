use super::super::RunHandle;
use crate::modules::pipeline::runtime::chat::{ChatContent, PatrolChatSink};
use crate::modules::pipeline::runtime::engine::patrol_state::{
    DispatchOutcome, EffectOutcome, PatrolDecisionEvidence,
};
use crate::modules::pipeline::runtime::patrol::{
    ActionEffectKind, ActionVerificationInput, PatrolDecision, PatrolRuntime,
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
}

pub(super) struct PatrolWorker {
    latest: Arc<std::sync::Mutex<Option<PatrolSnapshot>>>,
    action_frames: Arc<std::sync::Mutex<VecDeque<PatrolActionFrames>>>,
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
    ) -> Self {
        let latest = Arc::new(std::sync::Mutex::new(None::<PatrolSnapshot>));
        let observations = Arc::clone(&latest);
        let action_frames = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let worker_action_frames = Arc::clone(&action_frames);
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
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
            stop: Some(stop),
            task: Some(task),
        }
    }

    pub(super) fn observe(&self, snapshot: PatrolSnapshot) {
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
        if let Ok(mut attempts) = self.action_frames.lock() {
            attempts.push_back(attempt);
            while attempts.len() > 32 {
                attempts.pop_front();
            }
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
            if frames.expected_successors.is_empty()
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
    let screenshot = snapshot.screenshot.clone();
    let anchor_confirmed = snapshot.anchor_confirmed;
    let capture_target = snapshot.capture_target.clone();
    let capture_session = snapshot.capture_session;
    let frame_sequence = snapshot.frame_sequence;
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
            "trigger",
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
                Err(error) => (
                    None,
                    "inconclusive",
                    None,
                    Some(error.clone()),
                    EffectOutcome::Inconclusive {
                        session: capture_session,
                        frame: frame_sequence,
                        reason: error,
                    },
                ),
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
                "action_report",
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
                ],
            );
            let _ = app.emit("patrol-report", report);
            return;
        }
        let result = patrol
            .verify(&graph, &node_id, &screenshot, anchor_confirmed, &chat)
            .await;
        if !handle.patrol_reports_open() {
            log::debug!(
                "[patrol] discarding report for terminal run '{}'",
                handle.run_id
            );
            return;
        }
        let (matched_node_id, outcome, reason) = match result {
            Ok(PatrolDecision::Aligned) => (Some(node_id.clone()), "aligned", None),
            Ok(PatrolDecision::PossibleMismatch(candidate)) => {
                (Some(candidate), "possible_mismatch", None)
            }
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
            "report",
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

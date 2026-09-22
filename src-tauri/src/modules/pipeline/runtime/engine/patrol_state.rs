//! Per-run evidence for patrol. Sending input and observing its effect are
//! separate facts; a successful action plugin call never proves a GUI change.
use crate::modules::pipeline::authoring::models::action::Action;

const MAX_ACTIONS: usize = 512;
const MAX_ACTION_ATTEMPTS: usize = 128;
const MAX_OBSERVATIONS: usize = 256;
const MAX_DECISIONS: usize = 256;
const MAX_RECOVERIES: usize = 64;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PatrolState {
    pub actions: Vec<ActionEvidence>,
    pub action_attempts: Vec<ActionAttemptEvidence>,
    pub observations: Vec<ScreenObservation>,
    pub decisions: Vec<PatrolDecisionEvidence>,
    pub recoveries: Vec<RecoveryEvidence>,
    #[serde(skip)]
    next_action_id: u64,
    #[serde(skip)]
    next_attempt_id: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecoveryEvidence {
    pub policy: String,
    pub failure_node: String,
    pub target_node: Option<String>,
    pub decision: String,
    pub confidence: Option<f64>,
    pub reason: String,
    pub accepted: bool,
    pub redirect_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionAttemptEvidence {
    pub id: u64,
    pub node_id: String,
    pub action_ids: Vec<u64>,
    pub expected_successors: Vec<String>,
    pub error_successor: Option<String>,
    pub target: String,
    pub source_session: u64,
    pub before_frame: u64,
    pub after_frame: Option<u64>,
    pub started_ms: u64,
    pub completed_ms: Option<u64>,
    pub effect: EffectOutcome,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionEvidence {
    pub id: u64,
    pub attempt_id: u64,
    pub node_id: String,
    pub repetition: u32,
    pub action_index: usize,
    pub kind: String,
    /// Safe action geometry or counts. Input text, keys, and custom parameters
    /// are deliberately excluded from status and logs.
    pub intent: String,
    pub target: String,
    pub source_session: u64,
    pub source_frame: u64,
    pub started_ms: u64,
    pub completed_ms: Option<u64>,
    pub dispatch: DispatchOutcome,
    pub effect: EffectOutcome,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum DispatchOutcome {
    Pending,
    Sent,
    Failed(String),
    Blocked(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EffectOutcome {
    Unverified,
    Achieved {
        matched_successor: String,
        session: u64,
        frame: u64,
        confidence: f64,
        reason: String,
    },
    Unchanged {
        session: u64,
        frame: u64,
        confidence: f64,
        reason: String,
    },
    InProgress {
        session: u64,
        frame: u64,
        confidence: f64,
        reason: String,
    },
    Unexpected {
        matched_node: Option<String>,
        session: u64,
        frame: u64,
        confidence: f64,
        reason: String,
    },
    Inconclusive {
        session: u64,
        frame: u64,
        reason: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenObservation {
    pub expected_node_id: String,
    pub target: String,
    pub session: u64,
    pub frame: u64,
    pub elapsed_ms: u64,
    pub recognizer_matched: Option<bool>,
    pub recognizer_confidence: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PatrolDecisionEvidence {
    pub expected_node_id: String,
    pub matched_node_id: Option<String>,
    pub target: String,
    pub session: u64,
    pub frame: u64,
    pub elapsed_ms: u64,
    pub outcome: String,
    pub reason: Option<String>,
}

impl PatrolState {
    #[allow(clippy::too_many_arguments)]
    pub fn begin_attempt(
        &mut self,
        node_id: &str,
        expected_successors: Vec<String>,
        error_successor: Option<String>,
        target: &str,
        session: u64,
        before_frame: u64,
        elapsed_ms: u64,
    ) -> u64 {
        self.next_attempt_id += 1;
        let id = self.next_attempt_id;
        self.action_attempts.push(ActionAttemptEvidence {
            id,
            node_id: node_id.into(),
            action_ids: Vec::new(),
            expected_successors,
            error_successor,
            target: target.into(),
            source_session: session,
            before_frame,
            after_frame: None,
            started_ms: elapsed_ms,
            completed_ms: None,
            effect: EffectOutcome::Unverified,
        });
        trim(&mut self.action_attempts, MAX_ACTION_ATTEMPTS);
        id
    }

    #[allow(clippy::too_many_arguments)]
    pub fn begin_action(
        &mut self,
        attempt_id: u64,
        node_id: &str,
        repetition: u32,
        action_index: usize,
        action: &Action,
        target: &str,
        session: u64,
        frame: u64,
        elapsed_ms: u64,
    ) -> u64 {
        self.next_action_id += 1;
        let id = self.next_action_id;
        let (kind, intent) = summarize_action(action);
        self.actions.push(ActionEvidence {
            id,
            attempt_id,
            node_id: node_id.into(),
            repetition,
            action_index,
            kind: kind.into(),
            intent,
            target: target.into(),
            source_session: session,
            source_frame: frame,
            started_ms: elapsed_ms,
            completed_ms: None,
            dispatch: DispatchOutcome::Pending,
            effect: EffectOutcome::Unverified,
        });
        if let Some(attempt) = self
            .action_attempts
            .iter_mut()
            .find(|attempt| attempt.id == attempt_id)
        {
            attempt.action_ids.push(id);
        }
        trim(&mut self.actions, MAX_ACTIONS);
        id
    }

    pub fn finish_action(&mut self, id: u64, outcome: DispatchOutcome, elapsed_ms: u64) {
        if let Some(action) = self.actions.iter_mut().find(|action| action.id == id) {
            action.completed_ms = Some(elapsed_ms);
            action.dispatch = outcome;
        }
    }

    pub fn finish_attempt(&mut self, id: u64, elapsed_ms: u64) {
        if let Some(attempt) = self
            .action_attempts
            .iter_mut()
            .find(|attempt| attempt.id == id)
        {
            attempt.completed_ms = Some(elapsed_ms);
        }
    }

    pub fn apply_action_effect(
        &mut self,
        id: u64,
        target: &str,
        session: u64,
        frame: u64,
        effect: EffectOutcome,
    ) -> Result<(), String> {
        let attempt = self
            .action_attempts
            .iter_mut()
            .find(|attempt| attempt.id == id)
            .ok_or_else(|| format!("Unknown patrol action attempt {id}"))?;
        if attempt.target != target
            || attempt.source_session != session
            || frame <= attempt.before_frame
        {
            return Err("Action effect frame does not follow the attempt on the same target and capture session".into());
        }
        let effect_identity = match &effect {
            EffectOutcome::Unverified => {
                return Err("Cannot apply an unverified action effect as VLM evidence".into())
            }
            EffectOutcome::Achieved { session, frame, .. }
            | EffectOutcome::Unchanged { session, frame, .. }
            | EffectOutcome::InProgress { session, frame, .. }
            | EffectOutcome::Unexpected { session, frame, .. }
            | EffectOutcome::Inconclusive { session, frame, .. } => (*session, *frame),
        };
        if effect_identity != (session, frame) {
            return Err(
                "Action effect payload capture identity does not match its evidence frame".into(),
            );
        }
        if let EffectOutcome::Achieved {
            matched_successor, ..
        } = &effect
        {
            if !attempt.expected_successors.contains(matched_successor) {
                return Err(format!(
                    "Action effect matched unexpected successor '{matched_successor}'"
                ));
            }
        }
        if let EffectOutcome::Unexpected {
            matched_node: Some(matched_node),
            ..
        } = &effect
        {
            if attempt.error_successor.as_ref() != Some(matched_node) {
                return Err(format!(
                    "Action effect matched undeclared unexpected node '{matched_node}'"
                ));
            }
        }
        attempt.after_frame = Some(frame);
        attempt.effect = effect.clone();
        for action in &mut self.actions {
            if action.attempt_id == id && action.dispatch == DispatchOutcome::Sent {
                action.effect = effect.clone();
            }
        }
        Ok(())
    }

    pub fn observe(&mut self, observation: ScreenObservation) {
        self.observations.push(observation);
        trim(&mut self.observations, MAX_OBSERVATIONS);
    }

    pub fn recognize(
        &mut self,
        node_id: &str,
        session: u64,
        frame: u64,
        matched: bool,
        confidence: f64,
    ) {
        if let Some(observation) = self.observations.iter_mut().rev().find(|observation| {
            observation.expected_node_id == node_id
                && observation.session == session
                && observation.frame == frame
        }) {
            observation.recognizer_matched = Some(matched);
            observation.recognizer_confidence = Some(confidence);
        }
    }

    pub fn decide(&mut self, decision: PatrolDecisionEvidence) {
        self.decisions.push(decision);
        trim(&mut self.decisions, MAX_DECISIONS);
    }

    pub fn record_recovery(&mut self, recovery: RecoveryEvidence) {
        self.recoveries.push(recovery);
        trim(&mut self.recoveries, MAX_RECOVERIES);
    }
}

fn trim<T>(values: &mut Vec<T>, limit: usize) {
    if values.len() > limit {
        values.drain(..values.len() - limit);
    }
}

pub(crate) fn summarize_action(action: &Action) -> (&'static str, String) {
    match action {
        Action::DoNothing => ("DoNothing", String::new()),
        Action::Click { target, target_offset } => {
            ("Click", format!("target={target:?}, offset={target_offset:?}"))
        }
        Action::LongPress { target, target_offset, duration } => (
            "LongPress",
            format!("target={target:?}, offset={target_offset:?}, duration_ms={duration}"),
        ),
        Action::Swipe { begin, begin_offset, end, end_offset, duration } => (
            "Swipe",
            format!("begin={begin:?}, begin_offset={begin_offset:?}, end={end:?}, end_offset={end_offset:?}, duration_ms={duration}"),
        ),
        Action::Scroll { target, target_offset, dx, dy } => (
            "Scroll",
            format!("target={target:?}, offset={target_offset:?}, dx={dx}, dy={dy}"),
        ),
        Action::ClickKey { key } => {
            let controls = key
                .iter()
                .filter_map(|value| value.as_i64())
                .filter_map(control_key_name)
                .collect::<Vec<_>>();
            if controls.len() == key.len() && !controls.is_empty() {
                ("ClickKey", format!("controls={}", controls.join("+")))
            } else {
                ("ClickKey", format!("key_count={}", key.len()))
            }
        }
        Action::InputText { input_text } => (
            "InputText",
            format!("character_count={}", input_text.chars().count()),
        ),
        Action::StopTask => ("StopTask", String::new()),
        Action::Custom { action, .. } => ("Custom", format!("action={action}")),
    }
}

fn control_key_name(key: i64) -> Option<&'static str> {
    match key {
        8 => Some("Backspace"),
        9 => Some("Tab"),
        13 => Some("Enter"),
        27 => Some("Escape"),
        33 => Some("PageUp"),
        34 => Some("PageDown"),
        35 => Some("End"),
        36 => Some("Home"),
        37 => Some("ArrowLeft"),
        38 => Some("ArrowUp"),
        39 => Some("ArrowRight"),
        40 => Some("ArrowDown"),
        46 => Some("Delete"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent_attempt(state: &mut PatrolState) -> (u64, u64) {
        let attempt = state.begin_attempt(
            "a",
            vec!["b".into()],
            Some("error".into()),
            "window:7",
            1,
            4,
            10,
        );
        let action = state.begin_action(
            attempt,
            "a",
            0,
            0,
            &Action::InputText {
                input_text: "secret".into(),
            },
            "window:7",
            1,
            4,
            10,
        );
        state.finish_action(action, DispatchOutcome::Sent, 12);
        state.finish_attempt(attempt, 12);
        (attempt, action)
    }

    #[test]
    fn sending_input_and_state_matching_do_not_verify_its_effect() {
        let mut state = PatrolState::default();
        sent_attempt(&mut state);
        assert_eq!(state.actions[0].effect, EffectOutcome::Unverified);
        assert!(!serde_json::to_string(&state).unwrap().contains("secret"));
        state.decide(PatrolDecisionEvidence {
            expected_node_id: "b".into(),
            matched_node_id: Some("b".into()),
            target: "window:7".into(),
            session: 1,
            frame: 5,
            elapsed_ms: 20,
            outcome: "aligned".into(),
            reason: None,
        });
        assert_eq!(state.actions[0].effect, EffectOutcome::Unverified);
    }

    #[test]
    fn achieved_effect_requires_a_new_same_session_frame_and_expected_successor() {
        let mut state = PatrolState::default();
        let (attempt, _) = sent_attempt(&mut state);
        for (target, session, frame) in [("window:8", 1, 5), ("window:7", 2, 5), ("window:7", 1, 4)]
        {
            assert!(state
                .apply_action_effect(
                    attempt,
                    target,
                    session,
                    frame,
                    EffectOutcome::Achieved {
                        matched_successor: "b".into(),
                        session,
                        frame,
                        confidence: 0.9,
                        reason: "visible".into(),
                    },
                )
                .is_err());
        }
        assert_eq!(state.actions[0].effect, EffectOutcome::Unverified);
        assert!(state
            .apply_action_effect(
                attempt,
                "window:7",
                1,
                5,
                EffectOutcome::Achieved {
                    matched_successor: "other".into(),
                    session: 1,
                    frame: 5,
                    confidence: 0.9,
                    reason: "visible".into(),
                },
            )
            .is_err());
        state
            .apply_action_effect(
                attempt,
                "window:7",
                1,
                5,
                EffectOutcome::Achieved {
                    matched_successor: "b".into(),
                    session: 1,
                    frame: 5,
                    confidence: 0.9,
                    reason: "visible".into(),
                },
            )
            .unwrap();
        assert!(matches!(
            state.actions[0].effect,
            EffectOutcome::Achieved { .. }
        ));
    }

    #[test]
    fn action_summaries_redact_text_keys_and_custom_parameters() {
        let mut state = PatrolState::default();
        sent_attempt(&mut state);
        let serialized = serde_json::to_string(&state).unwrap();
        assert!(!serialized.contains("secret"));
        assert!(serialized.contains("character_count=6"));
    }

    #[test]
    fn action_summary_names_allowlisted_control_keys() {
        let (kind, intent) = summarize_action(&Action::ClickKey {
            key: vec![serde_json::json!(27)],
        });
        assert_eq!(kind, "ClickKey");
        assert_eq!(intent, "controls=Escape");
        let (_, redacted) = summarize_action(&Action::ClickKey {
            key: vec![serde_json::json!(65)],
        });
        assert_eq!(redacted, "key_count=1");
    }
}

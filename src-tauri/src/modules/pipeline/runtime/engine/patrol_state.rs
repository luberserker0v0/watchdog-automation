//! Per-run evidence for patrol. Sending input and observing its effect are
//! separate facts; a successful action plugin call never proves a GUI change.
use crate::modules::pipeline::authoring::models::action::Action;

const MAX_ACTIONS: usize = 512;
const MAX_OBSERVATIONS: usize = 256;
const MAX_DECISIONS: usize = 256;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PatrolState {
    pub actions: Vec<ActionEvidence>,
    pub observations: Vec<ScreenObservation>,
    pub decisions: Vec<PatrolDecisionEvidence>,
    #[serde(skip)]
    next_action_id: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionEvidence {
    pub id: u64,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EffectOutcome {
    Unverified,
    Observed {
        node_id: String,
        session: u64,
        frame: u64,
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
    pub fn begin_action(
        &mut self,
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
        trim(&mut self.actions, MAX_ACTIONS);
        id
    }

    pub fn finish_action(&mut self, id: u64, outcome: DispatchOutcome, elapsed_ms: u64) {
        if let Some(action) = self.actions.iter_mut().find(|action| action.id == id) {
            action.completed_ms = Some(elapsed_ms);
            action.dispatch = outcome;
        }
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
        if let Some(node_id) = &decision.matched_node_id {
            for action in &mut self.actions {
                if action.dispatch == DispatchOutcome::Sent
                    && action.effect == EffectOutcome::Unverified
                    && action.target == decision.target
                    && action.source_session == decision.session
                    && action.source_frame < decision.frame
                {
                    action.effect = EffectOutcome::Observed {
                        node_id: node_id.clone(),
                        session: decision.session,
                        frame: decision.frame,
                    };
                }
            }
        }
        self.decisions.push(decision);
        trim(&mut self.decisions, MAX_DECISIONS);
    }

    /// Compact, sanitized context for the VLM. These are dispatch facts, not
    /// claims that the GUI changed.
    pub fn recent_action_context(&self, target: &str) -> Vec<String> {
        let mut recent: Vec<String> = self
            .actions
            .iter()
            .rev()
            .filter(|action| action.target == target)
            .take(4)
            .map(|action| {
                format!(
                    "node={} action={} {} dispatch={:?} effect={:?}",
                    action.node_id, action.kind, action.intent, action.dispatch, action.effect
                )
                .chars()
                .take(300)
                .collect()
            })
            .collect();
        recent.reverse();
        recent
    }
}

fn trim<T>(values: &mut Vec<T>, limit: usize) {
    if values.len() > limit {
        values.drain(..values.len() - limit);
    }
}

fn summarize_action(action: &Action) -> (&'static str, String) {
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
        Action::ClickKey { key } => ("ClickKey", format!("key_count={}", key.len())),
        Action::InputText { input_text } => (
            "InputText",
            format!("character_count={}", input_text.chars().count()),
        ),
        Action::StopTask => ("StopTask", String::new()),
        Action::Custom { action, .. } => ("Custom", format!("action={action}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sending_input_does_not_verify_its_effect() {
        let mut state = PatrolState::default();
        let id = state.begin_action(
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
        state.finish_action(id, DispatchOutcome::Sent, 12);
        assert_eq!(state.actions[0].effect, EffectOutcome::Unverified);
        assert!(!serde_json::to_string(&state).unwrap().contains("secret"));
        assert!(!state
            .recent_action_context("window:7")
            .join(" ")
            .contains("secret"));
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
        assert_eq!(
            state.actions[0].effect,
            EffectOutcome::Observed {
                node_id: "b".into(),
                session: 1,
                frame: 5
            }
        );
    }

    #[test]
    fn stale_or_other_target_cannot_verify_an_action() {
        let mut state = PatrolState::default();
        let id = state.begin_action("a", 0, 0, &Action::DoNothing, "window:7", 1, 4, 10);
        state.finish_action(id, DispatchOutcome::Sent, 11);
        for (target, session, frame) in [("window:8", 1, 5), ("window:7", 2, 5), ("window:7", 1, 4)]
        {
            state.decide(PatrolDecisionEvidence {
                expected_node_id: "b".into(),
                matched_node_id: Some("b".into()),
                target: target.into(),
                session,
                frame,
                elapsed_ms: 20,
                outcome: "aligned".into(),
                reason: None,
            });
        }
        assert_eq!(state.actions[0].effect, EffectOutcome::Unverified);
    }

    #[test]
    fn blocked_action_is_not_marked_observed() {
        let mut state = PatrolState::default();
        let id = state.begin_action("a", 0, 0, &Action::DoNothing, "window:7", 1, 4, 10);
        state.finish_action(
            id,
            DispatchOutcome::Blocked("foreground refused".into()),
            11,
        );
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
}

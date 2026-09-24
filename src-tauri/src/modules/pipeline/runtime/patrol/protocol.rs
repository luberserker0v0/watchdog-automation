use image::DynamicImage;
use std::collections::HashSet;

use super::config::RecoveryPolicy;
use super::search::{Candidate, CandidateScore};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocationSource {
    StateMatcher,
    ActionExpectedSuccessor,
    ActionErrorSuccessor,
}

impl LocationSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StateMatcher => "state_matcher",
            Self::ActionExpectedSuccessor => "action_expected_successor",
            Self::ActionErrorSuccessor => "action_error_successor",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LocatedState {
    pub node_id: String,
    pub confidence: f64,
    pub source: LocationSource,
    pub capture_target: String,
    pub capture_session: u64,
    pub frame_sequence: u64,
    pub frame_stable: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateMatchResult {
    pub node_id: String,
    pub confidence: f64,
    pub reason: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ScoreEnvelope {
    scores: Vec<CandidateScore>,
}

pub(super) fn parse_scores(content: &str) -> Result<Vec<CandidateScore>, String> {
    let trimmed = content.trim();
    let json = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .unwrap_or(trimmed)
        .trim();
    serde_json::from_str::<ScoreEnvelope>(json)
        .map(|value| value.scores)
        .map_err(|error| format!("Patrol VLM returned invalid scores JSON: {error}"))
}

pub(super) fn validate_scores(
    candidates: &[Candidate],
    scores: &[CandidateScore],
) -> Result<(), String> {
    let expected: HashSet<&str> = candidates
        .iter()
        .map(|candidate| candidate.node_id.as_str())
        .collect();
    let mut observed = HashSet::new();
    for result in scores {
        if !expected.contains(result.node_id.as_str()) || !observed.insert(result.node_id.as_str())
        {
            return Err(format!(
                "Patrol VLM returned an unknown or repeated node '{}'",
                result.node_id
            ));
        }
        if !result.score.is_finite() || !(0.0..=1.0).contains(&result.score) {
            return Err(format!(
                "Patrol VLM returned an invalid score for '{}'",
                result.node_id
            ));
        }
    }
    if observed.len() != expected.len() {
        return Err("Patrol VLM did not score every candidate".into());
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionEffectKind {
    #[serde(rename = "expected_successor")]
    Achieved,
    Unchanged,
    InProgress,
    #[serde(rename = "error_successor")]
    Unexpected,
    Inconclusive,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActionEffectResult {
    pub effect: ActionEffectKind,
    pub matched_node: Option<String>,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Clone)]
pub struct ActionVerificationInput {
    pub attempt_id: u64,
    pub node_id: String,
    pub actions: Vec<String>,
    pub expected_successors: Vec<String>,
    pub error_successor: Option<String>,
    pub before: std::sync::Arc<DynamicImage>,
    pub after: std::sync::Arc<DynamicImage>,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryDecision {
    Suggest,
    Redirect,
    NoRecovery,
    Inconclusive,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecoveryResult {
    pub decision: RecoveryDecision,
    pub target_node: Option<String>,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Clone)]
pub struct RecoveryInput {
    pub failure_node: String,
    pub failure_reason: String,
    pub after: std::sync::Arc<DynamicImage>,
    pub recent_nodes: Vec<String>,
    pub located_state: Option<LocatedState>,
    pub location_uncertain: bool,
}

pub(super) fn validate_recovery_result(
    result: RecoveryResult,
    policy: RecoveryPolicy,
    candidates: &[Candidate],
) -> Result<RecoveryResult, String> {
    if !result.confidence.is_finite() || !(0.0..=1.0).contains(&result.confidence) {
        return Err("Patrol recovery confidence must be between 0 and 1".into());
    }
    let expected = match policy {
        RecoveryPolicy::HintOnly => RecoveryDecision::Suggest,
        RecoveryPolicy::RedirectionOnly => RecoveryDecision::Redirect,
    };
    match (&result.decision, &result.target_node) {
        (decision, Some(node_id)) if *decision == expected => {
            if !candidates
                .iter()
                .any(|candidate| candidate.node_id == *node_id)
            {
                return Err(format!(
                    "Patrol recovery named node '{node_id}' outside the candidate list"
                ));
            }
        }
        (RecoveryDecision::NoRecovery | RecoveryDecision::Inconclusive, None) => {}
        (decision, _) if *decision != expected => {
            return Err("Patrol recovery decision does not match the configured policy".into())
        }
        (_, None) => return Err("Patrol recovery decision requires target_node".into()),
        (_, Some(_)) => {
            return Err(
                "Patrol no_recovery and inconclusive decisions require a null target_node".into(),
            )
        }
    }
    Ok(result)
}

pub(super) fn parse_action_effect(
    content: &str,
    expected_successors: &[String],
    error_successor: Option<&str>,
) -> Result<ActionEffectResult, String> {
    let trimmed = content.trim();
    let json = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .unwrap_or(trimmed)
        .trim();
    let result: ActionEffectResult = serde_json::from_str(json)
        .map_err(|error| format!("Patrol VLM returned invalid action-effect JSON: {error}"))?;
    if !result.confidence.is_finite() || !(0.0..=1.0).contains(&result.confidence) {
        return Err("Patrol VLM action-effect confidence must be between 0 and 1".into());
    }
    match (&result.effect, &result.matched_node) {
        (ActionEffectKind::Achieved, Some(node_id)) if expected_successors.contains(node_id) => {}
        (ActionEffectKind::Achieved, Some(node_id)) => {
            return Err(format!(
                "Patrol VLM expected_successor result named a node outside the normal successor set: '{node_id}'"
            ));
        }
        (ActionEffectKind::Achieved, None) => {
            return Err("Patrol VLM expected_successor result requires matched_node".into())
        }
        (ActionEffectKind::Unexpected, Some(node_id)) if error_successor == Some(node_id) => {}
        (ActionEffectKind::Unexpected, Some(node_id)) => {
            return Err(format!(
                "Patrol VLM error_successor result named a node other than the declared error successor: '{node_id}'"
            ));
        }
        (_, Some(_)) => {
            return Err(
                "Patrol VLM matched_node is allowed only for expected_successor or error_successor"
                    .into(),
            )
        }
        _ => {}
    }
    Ok(result)
}

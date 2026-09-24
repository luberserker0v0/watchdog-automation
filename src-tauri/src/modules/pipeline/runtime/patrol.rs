mod config;
mod protocol;
mod references;
mod search;
mod vlm;

pub use config::{PatrolConfig, RecoveryPolicy, VlmConnection};
pub use protocol::{
    ActionEffectKind, ActionEffectResult, ActionVerificationInput, LocatedState, LocationSource,
    RecoveryDecision, RecoveryInput, RecoveryResult, StateMatchResult,
};
pub use references::ReferenceStore;
pub use search::{
    bounded_candidates, candidate_tiers, recovery_candidates, select_confident_match, Candidate,
    CandidateScore, CandidateTiers,
};
use vlm::{VlmError, VlmScorer};

#[cfg(test)]
use protocol::{parse_action_effect, parse_scores, validate_recovery_result, validate_scores};
use references::encode_frame;
use search::{bounded_candidates as bound_candidates, inconclusive_scores, select_match};
// Bounded local candidate search for visual state patrol. Edges are treated as
// undirected because a missed transition may leave the UI at a nearby state.
use crate::modules::pipeline::authoring::models::graph::PipelineGraph;
#[cfg(test)]
use crate::modules::pipeline::authoring::models::node::NodeKind;
use crate::modules::pipeline::runtime::chat::{
    cache_image, ChatContent, PatrolChatKind, PatrolChatSink,
};
use image::DynamicImage;
use std::path::Path;

pub struct PatrolRuntime {
    pub config: PatrolConfig,
    pub store: ReferenceStore,
    scorer: VlmScorer,
    transport_health: std::sync::Mutex<TransportHealth>,
}

#[derive(Default)]
struct TransportHealth {
    consecutive_failures: u8,
    circuit_open: bool,
    circuit_reported: bool,
    final_probe_used: bool,
    last_transport_failure: bool,
}

impl PatrolRuntime {
    pub fn new(
        config: PatrolConfig,
        pipeline_path: &Path,
        run_id: &str,
        connection: Option<VlmConnection>,
    ) -> Result<Self, String> {
        config.validate()?;
        let store = ReferenceStore::for_run(pipeline_path, run_id)?;
        let scorer = VlmScorer::new(
            connection.ok_or("Enabled patrol requires an LLM endpoint and model in Settings")?,
        )?;
        Ok(Self {
            config,
            store,
            scorer,
            transport_health: std::sync::Mutex::new(TransportHealth::default()),
        })
    }

    fn track<T>(&self, result: Result<T, VlmError>) -> Result<T, String> {
        if let Ok(mut health) = self.transport_health.lock() {
            match &result {
                Ok(_) | Err(VlmError::Protocol(_)) => {
                    health.consecutive_failures = 0;
                    health.last_transport_failure = false;
                }
                Err(error) if error.is_transport() => {
                    health.last_transport_failure = true;
                    health.consecutive_failures = health.consecutive_failures.saturating_add(1);
                    if health.consecutive_failures >= 3 {
                        health.circuit_open = true;
                    }
                }
                Err(_) => {}
            }
        }
        result.map_err(|error| error.to_string())
    }

    pub fn periodic_allowed(&self) -> bool {
        self.transport_health
            .lock()
            .map(|health| !health.circuit_open)
            .unwrap_or(false)
    }

    pub fn take_circuit_report(&self) -> bool {
        self.transport_health.lock().is_ok_and(|mut health| {
            if health.circuit_open && !health.circuit_reported {
                health.circuit_reported = true;
                true
            } else {
                false
            }
        })
    }

    pub fn allow_final_probe(&self) -> bool {
        self.transport_health.lock().is_ok_and(|mut health| {
            if !health.circuit_open {
                true
            } else if !health.final_probe_used {
                health.final_probe_used = true;
                true
            } else {
                false
            }
        })
    }

    pub fn last_request_was_transport_failure(&self) -> bool {
        self.transport_health
            .lock()
            .map(|health| health.last_transport_failure)
            .unwrap_or(true)
    }

    pub fn record(&self, node_id: &str, frame: &DynamicImage) -> Result<(String, Vec<u8>), String> {
        self.store.save(node_id, frame)
    }

    pub async fn verify(
        &self,
        graph: &PipelineGraph,
        anchor: &str,
        frame: &DynamicImage,
        anchor_confirmed: bool,
        recent_nodes: &[String],
        chat: &PatrolChatSink,
    ) -> Result<StateMatchResult, String> {
        let frame_bytes = encode_frame(frame)?;
        let frame_image_id = cache_image(frame_bytes.clone());
        chat.emit(
            "patrol",
            PatrolChatKind::Capture,
            format!("Captured {} for {}", chat.target(), anchor),
            None,
            vec![
                ChatContent::Field {
                    label: "Expected node".into(),
                    value: anchor.into(),
                },
                ChatContent::Image {
                    label: "Current capture".into(),
                    image_id: frame_image_id,
                },
            ],
        );
        let mut tiers = candidate_tiers(graph, anchor, self.config.k_hops);
        tiers.local = bound_candidates(graph, anchor, tiers.local, recent_nodes, 16);
        if tiers.local.is_empty() {
            return Err(format!(
                "Patrol has no visual candidates near node '{anchor}'"
            ));
        }
        chat.emit(
            "patrol",
            PatrolChatKind::Search,
            format!("Searching {} nearby nodes", tiers.local.len()),
            Some("local"),
            tiers
                .local
                .iter()
                .map(|candidate| ChatContent::Field {
                    label: candidate.node_id.clone(),
                    value: format!("{} hops", candidate.hops),
                })
                .collect(),
        );
        let local_scores = self.track(
            self.scorer
                .score_candidates(
                    &self.store,
                    graph,
                    anchor,
                    &tiers.local,
                    &frame_bytes,
                    "local",
                    chat,
                )
                .await,
        )?;
        log::info!("[patrol] anchor='{anchor}' tier=local scores={local_scores:?}");
        if let Some(node_id) = select_match(
            &tiers.local,
            &local_scores,
            self.config.threshold,
            self.config.min_margin,
            anchor_confirmed.then_some(anchor),
        )? {
            return Ok(Self::state_match_result(node_id, &local_scores));
        }
        if tiers.expanded.is_empty() {
            return Err(format!(
                "Patrol could not confidently match node '{anchor}' in its local neighborhood ({})",
                inconclusive_scores(&local_scores, self.config.threshold, self.config.min_margin)
            ));
        }
        let mut expanded = tiers.local;
        expanded.extend(tiers.expanded);
        let expanded = bound_candidates(graph, anchor, expanded, recent_nodes, 16);
        chat.emit(
            "patrol",
            PatrolChatKind::Expansion,
            format!("Expanding search to {} nodes", expanded.len()),
            Some("expanded"),
            expanded
                .iter()
                .map(|candidate| ChatContent::Field {
                    label: candidate.node_id.clone(),
                    value: format!("{} hops", candidate.hops),
                })
                .collect(),
        );
        let expanded_scores = self.track(
            self.scorer
                .score_candidates(
                    &self.store,
                    graph,
                    anchor,
                    &expanded,
                    &frame_bytes,
                    "expanded",
                    chat,
                )
                .await,
        )?;
        log::info!("[patrol] anchor='{anchor}' tier=expanded scores={expanded_scores:?}");
        let node_id = select_match(
            &expanded,
            &expanded_scores,
            self.config.threshold,
            self.config.min_margin,
            anchor_confirmed.then_some(anchor),
        )?
        .ok_or_else(|| {
            format!(
                "Patrol could not confidently match node '{anchor}' after expansion ({})",
                inconclusive_scores(
                    &expanded_scores,
                    self.config.threshold,
                    self.config.min_margin
                )
            )
        })?;
        Ok(Self::state_match_result(node_id, &expanded_scores))
    }

    pub async fn verify_action_effect(
        &self,
        graph: &PipelineGraph,
        input: &ActionVerificationInput,
        chat: &PatrolChatSink,
    ) -> Result<ActionEffectResult, String> {
        let mut result = self.track(
            self.scorer
                .verify_action_effect(&self.store, graph, input, chat)
                .await,
        )?;
        normalize_action_effect(&mut result, self.config.action_effect_threshold());
        Ok(result)
    }

    pub async fn assess_recovery(
        &self,
        graph: &PipelineGraph,
        input: &RecoveryInput,
        chat: &PatrolChatSink,
    ) -> Result<RecoveryResult, String> {
        let anchor = recovery_anchor(input, self.config.recovery_policy)?;
        let candidates =
            recovery_candidates(graph, anchor, &input.recent_nodes, self.config.k_hops);
        if candidates.is_empty() {
            return Err("Patrol recovery has no eligible candidate nodes".into());
        }
        self.track(
            self.scorer
                .assess_recovery(
                    &self.store,
                    graph,
                    input,
                    &candidates,
                    self.config.recovery_policy,
                    chat,
                )
                .await,
        )
    }

    fn state_match_result(node_id: String, scores: &[CandidateScore]) -> StateMatchResult {
        let confidence = scores
            .iter()
            .find(|score| score.node_id == node_id)
            .map_or(0.0, |score| score.score);
        StateMatchResult {
            reason: format!("State Matcher selected '{node_id}' with confidence {confidence:.2}"),
            node_id,
            confidence,
        }
    }
}

fn recovery_anchor(input: &RecoveryInput, policy: RecoveryPolicy) -> Result<&str, String> {
    match (&input.located_state, policy) {
        (Some(location), _) => Ok(&location.node_id),
        (None, RecoveryPolicy::HintOnly) => Ok(&input.failure_node),
        (None, RecoveryPolicy::RedirectionOnly) => {
            Err("Patrol recovery location is unresolved; redirection is not allowed".into())
        }
    }
}

fn normalize_action_effect(result: &mut ActionEffectResult, threshold: f64) {
    if matches!(
        result.effect,
        ActionEffectKind::Achieved | ActionEffectKind::Unexpected
    ) && result.confidence < threshold
    {
        let original = format!("{:?}", result.effect);
        result.effect = ActionEffectKind::Inconclusive;
        result.matched_node = None;
        result.reason = format!(
            "Low-confidence {original} result ({:.2} < {:.2}): {}",
            result.confidence, threshold, result.reason
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::pipeline::authoring::models::edge::{Edge, EdgeType};
    use crate::modules::pipeline::authoring::models::node::{Node, NodeConfig};

    #[test]
    fn enabled_patrol_requires_vlm_configuration() {
        let default = PatrolConfig::default();
        assert!(!default.enabled);
        assert_eq!(default.cycle_seconds, 20);
        assert_eq!(default.recovery_policy, RecoveryPolicy::HintOnly);
        assert_eq!(default.max_redirects, 1);
        let legacy: PatrolConfig = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert_eq!(legacy.cycle_seconds, 20);
        assert_eq!(legacy.action_effect_threshold(), legacy.threshold);
        assert_eq!(legacy.redirect_threshold(), legacy.threshold);
        assert!(PatrolConfig {
            cycle_seconds: 0,
            ..default.clone()
        }
        .validate()
        .is_err());
        assert!(PatrolConfig {
            max_redirects: 0,
            ..default.clone()
        }
        .validate()
        .is_err());
        let temp = tempfile::tempdir().unwrap();
        let pipeline = temp.path().join("pipeline.md");
        std::fs::write(&pipeline, "flowchart TD").unwrap();
        let runtime = PatrolRuntime::new(
            PatrolConfig {
                enabled: true,
                ..default
            },
            &pipeline,
            "a1",
            None,
        );
        assert!(runtime.is_err());
    }

    fn graph() -> PipelineGraph {
        let node = |id: &str, kind| Node {
            id: id.into(),
            label: id.into(),
            config: NodeConfig {
                kind,
                ..Default::default()
            },
            params: Vec::new(),
        };
        let edge = |from: &str, to: &str| Edge {
            from: from.into(),
            to: to.into(),
            edge_type: EdgeType::Next,
        };
        PipelineGraph {
            version: "1".into(),
            target: None,
            launch: None,
            nodes: vec![
                node("Start", NodeKind::Entry),
                node("A", NodeKind::Normal),
                node("B", NodeKind::Normal),
                node("C", NodeKind::Normal),
                node("D", NodeKind::Normal),
            ],
            edges: vec![
                edge("Start", "A"),
                edge("A", "B"),
                edge("A", "C"),
                edge("C", "D"),
            ],
        }
    }

    #[test]
    fn local_then_expanded_search_includes_shared_parent_siblings() {
        let tiers = candidate_tiers(&graph(), "B", 1);
        assert_eq!(
            tiers
                .local
                .iter()
                .map(|c| c.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["B", "A"]
        );
        assert_eq!(
            tiers
                .expanded
                .iter()
                .map(|c| c.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["C"]
        );
    }

    #[test]
    fn unknown_anchor_has_no_candidates() {
        let tiers = candidate_tiers(&graph(), "missing", 2);
        assert!(tiers.local.is_empty());
        assert!(tiers.expanded.is_empty());
    }

    #[test]
    fn candidate_requests_are_bounded_and_priority_ordered() {
        let node = |id: String| Node {
            label: id.clone(),
            id,
            config: NodeConfig {
                kind: NodeKind::Normal,
                ..Default::default()
            },
            params: Vec::new(),
        };
        let graph = PipelineGraph {
            version: "1".into(),
            target: None,
            launch: None,
            nodes: (0..24).map(|index| node(format!("N{index}"))).collect(),
            edges: Vec::new(),
        };
        let candidates = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(hops, node)| Candidate {
                node_id: node.id.clone(),
                hops,
            })
            .collect();
        let bounded = bounded_candidates(&graph, "N0", candidates, &["N23".into()], 16);
        assert_eq!(bounded.len(), 16);
        assert_eq!(bounded[0].node_id, "N0");
        assert_eq!(bounded[1].node_id, "N23");
        assert_eq!(bounded[2].node_id, "N1");
        let recovery = recovery_candidates(&graph, "N0", &["N23".into()], 1);
        assert_eq!(recovery.len(), 16);
        assert_eq!(recovery[0].node_id, "N0");
        assert_eq!(recovery[1].node_id, "N23");
    }

    #[test]
    fn recovery_candidates_start_with_location_then_prioritize_route() {
        let candidates =
            recovery_candidates(&graph(), "B", &["A".into(), "C".into(), "B".into()], 1);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["B", "A", "C", "D"]
        );
    }

    #[test]
    fn recovery_response_is_policy_and_candidate_bounded() {
        let candidates = vec![Candidate {
            node_id: "A".into(),
            hops: 1,
        }];
        let hint = RecoveryResult {
            decision: RecoveryDecision::Suggest,
            target_node: Some("A".into()),
            confidence: 0.9,
            reason: "retry input".into(),
        };
        assert!(validate_recovery_result(hint, RecoveryPolicy::HintOnly, &candidates).is_ok());
        let redirect = RecoveryResult {
            decision: RecoveryDecision::Redirect,
            target_node: Some("A".into()),
            confidence: 0.9,
            reason: "retry input".into(),
        };
        assert!(
            validate_recovery_result(redirect.clone(), RecoveryPolicy::HintOnly, &candidates)
                .is_err()
        );
        assert!(
            validate_recovery_result(redirect, RecoveryPolicy::RedirectionOnly, &candidates)
                .is_ok()
        );
        assert!(validate_recovery_result(
            RecoveryResult {
                decision: RecoveryDecision::Redirect,
                target_node: Some("outside".into()),
                confidence: 1.0,
                reason: "bad".into(),
            },
            RecoveryPolicy::RedirectionOnly,
            &candidates,
        )
        .is_err());
    }

    #[test]
    fn unresolved_location_allows_hints_but_blocks_redirection() {
        let input = RecoveryInput {
            failure_node: "B".into(),
            failure_reason: "failed".into(),
            after: std::sync::Arc::new(DynamicImage::new_rgb8(2, 2)),
            recent_nodes: vec!["A".into()],
            located_state: None,
            location_uncertain: true,
        };
        assert_eq!(recovery_anchor(&input, RecoveryPolicy::HintOnly), Ok("B"));
        assert!(recovery_anchor(&input, RecoveryPolicy::RedirectionOnly).is_err());
    }

    #[test]
    fn scoring_rejects_ambiguous_or_incomplete_model_output() {
        let candidates = candidate_tiers(&graph(), "B", 1).local;
        let scores = vec![
            CandidateScore {
                node_id: "B".into(),
                score: 0.86,
            },
            CandidateScore {
                node_id: "A".into(),
                score: 0.83,
            },
        ];
        assert_eq!(
            select_confident_match(&candidates, &scores, 0.8, 0.1).unwrap(),
            None
        );
        assert!(select_confident_match(&candidates, &scores[..1], 0.8, 0.1).is_err());
        let confident = vec![
            CandidateScore {
                node_id: "B".into(),
                score: 0.92,
            },
            CandidateScore {
                node_id: "A".into(),
                score: 0.2,
            },
        ];
        assert_eq!(
            select_confident_match(&candidates, &confident, 0.8, 0.1).unwrap(),
            Some("B".into())
        );
    }

    #[test]
    fn matched_recognizer_allows_top_anchor_when_other_gui_elements_coexist() {
        let candidates = candidate_tiers(&graph(), "B", 2).local;
        let scores = vec![
            CandidateScore {
                node_id: "B".into(),
                score: 0.98,
            },
            CandidateScore {
                node_id: "A".into(),
                score: 0.97,
            },
            CandidateScore {
                node_id: "C".into(),
                score: 0.96,
            },
            CandidateScore {
                node_id: "Start".into(),
                score: 0.95,
            },
        ];
        let scores: Vec<_> = scores
            .into_iter()
            .filter(|score| {
                candidates
                    .iter()
                    .any(|candidate| candidate.node_id == score.node_id)
            })
            .collect();
        assert_eq!(
            select_confident_match(&candidates, &scores, 0.85, 0.1).unwrap(),
            None
        );
        assert_eq!(
            select_match(&candidates, &scores, 0.85, 0.1, Some("B")).unwrap(),
            Some("B".into())
        );

        let mut other_higher = scores.clone();
        other_higher
            .iter_mut()
            .find(|score| score.node_id == "A")
            .unwrap()
            .score = 0.99;
        assert_eq!(
            select_match(&candidates, &other_higher, 0.85, 0.1, Some("B")).unwrap(),
            None
        );

        let below_threshold: Vec<_> = scores
            .iter()
            .map(|score| CandidateScore {
                node_id: score.node_id.clone(),
                score: score.score - 0.2,
            })
            .collect();
        assert_eq!(
            select_match(&candidates, &below_threshold, 0.85, 0.1, Some("B")).unwrap(),
            None
        );
    }

    #[test]
    fn parses_model_scores_with_or_without_markdown_fence() {
        let body = r#"{"scores":[{"node_id":"A","score":0.9}]}"#;
        assert_eq!(parse_scores(body).unwrap()[0].node_id, "A");
        assert_eq!(
            parse_scores(&format!("```json\n{body}\n```")).unwrap()[0].score,
            0.9
        );
        assert!(parse_scores("I think A matches").is_err());
        assert!(parse_scores(r#"{"scores":[{"node_id":"A","score":0.9,"extra":true}]}"#).is_err());
        let candidates = vec![Candidate {
            node_id: "A".into(),
            hops: 0,
        }];
        assert!(validate_scores(&candidates, &[]).is_err());
        assert!(validate_scores(
            &candidates,
            &[CandidateScore {
                node_id: "other".into(),
                score: 0.9
            }]
        )
        .is_err());
        assert!(validate_scores(
            &candidates,
            &[CandidateScore {
                node_id: "A".into(),
                score: 1.1
            }]
        )
        .is_err());
    }

    #[test]
    fn action_effect_response_is_strict_and_successor_bounded() {
        let successors = vec!["Done".to_string()];
        let achieved = parse_action_effect(
            r#"{"effect":"expected_successor","matched_node":"Done","confidence":0.93,"reason":"success view"}"#,
            &successors,
            Some("Error"),
        )
        .unwrap();
        assert_eq!(achieved.effect, ActionEffectKind::Achieved);
        assert_eq!(achieved.matched_node.as_deref(), Some("Done"));
        assert!(parse_action_effect(
            r#"{"effect":"expected_successor","matched_node":null,"confidence":0.9,"reason":"missing"}"#,
            &successors,
            Some("Error"),
        )
        .is_err());
        assert!(parse_action_effect(
            r#"{"effect":"unchanged","matched_node":"Done","confidence":0.9,"reason":"still there"}"#,
            &successors,
            Some("Error"),
        ).is_err());
        assert!(parse_action_effect(
            r#"{"effect":"expected_successor","matched_node":"Other","confidence":0.9,"reason":"wrong"}"#,
            &successors,
            Some("Error"),
        )
        .is_err());
        let error = parse_action_effect(
            r#"{"effect":"error_successor","matched_node":"Error","confidence":0.95,"reason":"error visible"}"#,
            &successors,
            Some("Error"),
        )
        .unwrap();
        assert_eq!(error.effect, ActionEffectKind::Unexpected);
        assert_eq!(error.matched_node.as_deref(), Some("Error"));
        assert!(parse_action_effect(
            r#"{"effect":"achieved","matched_node":"Error","confidence":1.0,"reason":"transition happened"}"#,
            &successors,
            Some("Error"),
        )
        .is_err());
        assert!(parse_action_effect(
            r#"{"effect":"in_progress","matched_node":null,"confidence":2.0,"reason":"loading"}"#,
            &successors,
            Some("Error"),
        )
        .is_err());
        assert!(parse_action_effect(
            r#"{"effect":"unknown","matched_node":null,"confidence":0.5,"reason":"bad enum"}"#,
            &successors,
            Some("Error"),
        )
        .is_err());
        assert!(parse_action_effect(
            r#"{"effect":"inconclusive","matched_node":null,"confidence":0.0,"reason":"unclear","extra":true}"#,
            &successors,
            Some("Error"),
        ).is_err());
    }

    #[test]
    fn low_confidence_successor_results_become_inconclusive() {
        for effect in [ActionEffectKind::Achieved, ActionEffectKind::Unexpected] {
            let mut result = ActionEffectResult {
                effect,
                matched_node: Some("Next".into()),
                confidence: 0.7,
                reason: "weak evidence".into(),
            };
            normalize_action_effect(&mut result, 0.85);
            assert_eq!(result.effect, ActionEffectKind::Inconclusive);
            assert!(result.matched_node.is_none());
            assert!(result.reason.contains("Low-confidence"));
        }
    }

    #[test]
    fn transport_circuit_opens_once_and_allows_one_final_probe() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = temp.path().join("pipeline.md");
        std::fs::write(&pipeline, "flowchart TD").unwrap();
        let runtime = PatrolRuntime::new(
            PatrolConfig::default(),
            &pipeline,
            "abcdef1234567890",
            Some(VlmConnection {
                endpoint: "http://127.0.0.1:1/v1/chat/completions".into(),
                model: "vision".into(),
                api_key: None,
            }),
        )
        .unwrap();
        for _ in 0..3 {
            let _ = runtime.track::<()>(Err(VlmError::Transport(
                vlm::TransportFailureKind::Connection,
                "offline".into(),
            )));
        }
        assert!(!runtime.periodic_allowed());
        assert!(runtime.take_circuit_report());
        assert!(!runtime.take_circuit_report());
        assert!(runtime.allow_final_probe());
        assert!(!runtime.allow_final_probe());
    }

    #[test]
    fn reference_frames_are_unannotated_and_can_be_loaded() {
        let temp = tempfile::tempdir().unwrap();
        let store = ReferenceStore {
            root: temp.path().to_path_buf(),
        };
        let frame = DynamicImage::new_rgb8(20, 12);
        assert!(store.load("A").unwrap().is_none());
        store.save("A", &frame).unwrap();
        let bytes = store.load("A").unwrap().unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (1280, 768));
        assert!(store.load("../escape").is_err());
    }

    #[test]
    fn references_are_isolated_by_run() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = temp.path().join("sample.md");
        std::fs::write(&pipeline, "flowchart TD\nStart --> Done").unwrap();
        let base = temp.path().join("patrol");
        let first = ReferenceStore::for_run_in(base.clone(), &pipeline, "a1").unwrap();
        let second = ReferenceStore::for_run_in(base, &pipeline, "b2").unwrap();
        let legacy = ReferenceStore {
            root: first.root.parent().unwrap().to_path_buf(),
        };
        legacy
            .save("Done", &DynamicImage::new_rgb8(20, 12))
            .unwrap();
        first
            .save("Start", &DynamicImage::new_rgb8(20, 12))
            .unwrap();
        assert!(first.load("Start").unwrap().is_some());
        assert!(second.load("Start").unwrap().is_none());
        assert!(second.load("Done").unwrap().is_none());
        assert_ne!(first.root, second.root);
        assert!(
            ReferenceStore::for_run_in(temp.path().to_path_buf(), &pipeline, "../escape").is_err()
        );
    }

    #[test]
    fn state_match_result_preserves_selected_confidence() {
        let result = PatrolRuntime::state_match_result(
            "B".into(),
            &[
                CandidateScore {
                    node_id: "A".into(),
                    score: 0.1,
                },
                CandidateScore {
                    node_id: "B".into(),
                    score: 0.93,
                },
            ],
        );
        assert_eq!(result.node_id, "B");
        assert_eq!(result.confidence, 0.93);
        assert!(result.reason.contains("0.93"));
    }
}

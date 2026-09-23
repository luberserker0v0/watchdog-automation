mod config;
mod protocol;
mod references;
mod search;
mod vlm;

pub use config::{PatrolConfig, RecoveryPolicy, VlmConnection};
pub use protocol::{
    ActionEffectKind, ActionEffectResult, ActionVerificationInput, RecoveryDecision, RecoveryInput,
    RecoveryResult,
};
pub use references::ReferenceStore;
pub use search::{
    candidate_tiers, recovery_candidates, select_confident_match, Candidate, CandidateScore,
    CandidateTiers,
};
use vlm::VlmScorer;

#[cfg(test)]
use protocol::{parse_action_effect, parse_scores, validate_recovery_result, validate_scores};
use references::encode_frame;
use search::{inconclusive_scores, select_match};
// Bounded local candidate search for visual state patrol. Edges are treated as
// undirected because a missed transition may leave the UI at a nearby state.
use crate::modules::pipeline::authoring::models::graph::PipelineGraph;
#[cfg(test)]
use crate::modules::pipeline::authoring::models::node::NodeKind;
use crate::modules::pipeline::runtime::chat::{cache_image, ChatContent, PatrolChatSink};
use image::DynamicImage;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatrolDecision {
    Aligned,
    PossibleMismatch(String),
}

pub struct PatrolRuntime {
    pub config: PatrolConfig,
    pub store: ReferenceStore,
    scorer: VlmScorer,
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
        })
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
        chat: &PatrolChatSink,
    ) -> Result<PatrolDecision, String> {
        let frame_bytes = encode_frame(frame)?;
        let frame_image_id = cache_image(frame_bytes.clone());
        chat.emit(
            "patrol",
            "capture",
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
        let tiers = candidate_tiers(graph, anchor, self.config.k_hops);
        if tiers.local.is_empty() {
            return Err(format!(
                "Patrol has no visual candidates near node '{anchor}'"
            ));
        }
        chat.emit(
            "patrol",
            "search",
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
        let local_scores = self
            .scorer
            .score_candidates(
                &self.store,
                graph,
                anchor,
                &tiers.local,
                &frame_bytes,
                "local",
                chat,
            )
            .await?;
        log::info!("[patrol] anchor='{anchor}' tier=local scores={local_scores:?}");
        if let Some(node_id) = select_match(
            &tiers.local,
            &local_scores,
            self.config.threshold,
            self.config.min_margin,
            anchor_confirmed.then_some(anchor),
        )? {
            return Ok(Self::decision_for(anchor, node_id));
        }
        if tiers.expanded.is_empty() {
            return Err(format!(
                "Patrol could not confidently match node '{anchor}' in its local neighborhood ({})",
                inconclusive_scores(&local_scores, self.config.threshold, self.config.min_margin)
            ));
        }
        let mut expanded = tiers.local;
        expanded.extend(tiers.expanded);
        chat.emit(
            "patrol",
            "expansion",
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
        let expanded_scores = self
            .scorer
            .score_candidates(
                &self.store,
                graph,
                anchor,
                &expanded,
                &frame_bytes,
                "expanded",
                chat,
            )
            .await?;
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
        Ok(Self::decision_for(anchor, node_id))
    }

    pub async fn verify_action_effect(
        &self,
        graph: &PipelineGraph,
        input: &ActionVerificationInput,
        chat: &PatrolChatSink,
    ) -> Result<ActionEffectResult, String> {
        self.scorer
            .verify_action_effect(&self.store, graph, input, chat)
            .await
    }

    pub async fn assess_recovery(
        &self,
        graph: &PipelineGraph,
        input: &RecoveryInput,
        chat: &PatrolChatSink,
    ) -> Result<RecoveryResult, String> {
        let candidates = recovery_candidates(
            graph,
            &input.failure_node,
            &input.recent_nodes,
            self.config.k_hops,
        );
        if candidates.is_empty() {
            return Err("Patrol recovery has no eligible candidate nodes".into());
        }
        self.scorer
            .assess_recovery(
                &self.store,
                graph,
                input,
                &candidates,
                self.config.recovery_policy,
                chat,
            )
            .await
    }

    fn decision_for(anchor: &str, node_id: String) -> PatrolDecision {
        if node_id == anchor {
            PatrolDecision::Aligned
        } else {
            PatrolDecision::PossibleMismatch(node_id)
        }
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
    fn recovery_candidates_prioritize_recent_nodes_and_exclude_failure() {
        let candidates =
            recovery_candidates(&graph(), "B", &["A".into(), "C".into(), "B".into()], 1);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["C", "A"]
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
    fn description_only_first_visit_can_select_a_node() {
        assert_eq!(
            PatrolRuntime::decision_for("A", "B".into()),
            PatrolDecision::PossibleMismatch("B".into())
        );
        assert_eq!(
            PatrolRuntime::decision_for("A", "A".into()),
            PatrolDecision::Aligned
        );
    }
}

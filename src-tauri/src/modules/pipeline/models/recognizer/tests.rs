use super::*;
use serde_json;

#[test]
fn test_direct_hit_serde() {
    let recognition = Recognition::DirectHit;
    let json = serde_json::to_string(&recognition).unwrap();
    assert!(json.contains("\"type\":\"DirectHit\""));

    let deserialized: Recognition = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Recognition::DirectHit));
}

#[test]
fn test_template_match_serde() {
    let recognition = Recognition::TemplateMatch {
        template: "test.png".to_string(),
        threshold: Some(0.8),
    };

    let json = serde_json::to_string(&recognition).unwrap();
    assert!(json.contains("\"type\":\"TemplateMatch\""));
    assert!(json.contains("\"template\":\"test.png\""));
    assert!(json.contains("\"threshold\":0.8"));

    let deserialized: Recognition = serde_json::from_str(&json).unwrap();
    match deserialized {
        Recognition::TemplateMatch {
            template,
            threshold,
        } => {
            assert_eq!(template, "test.png");
            assert_eq!(threshold, Some(0.8));
        }
        _ => panic!("Expected TemplateMatch recognition"),
    }
}

#[test]
fn test_ocr_serde() {
    let recognition = Recognition::Ocr {
        expected: Some("Hello".to_string()),
        threshold: Some(0.5),
    };

    let json = serde_json::to_string(&recognition).unwrap();
    assert!(json.contains("\"type\":\"OCR\""));

    let deserialized: Recognition = serde_json::from_str(&json).unwrap();
    match deserialized {
        Recognition::Ocr {
            expected,
            threshold,
        } => {
            assert_eq!(expected, Some("Hello".to_string()));
            assert_eq!(threshold, Some(0.5));
        }
        _ => panic!("Expected OCR recognition"),
    }
}

#[test]
fn test_color_match_serde() {
    let recognition = Recognition::ColorMatch {
        lower: vec![0, 0, 0],
        upper: vec![255, 255, 255],
    };

    let json = serde_json::to_string(&recognition).unwrap();
    assert!(json.contains("\"type\":\"ColorMatch\""));

    let deserialized: Recognition = serde_json::from_str(&json).unwrap();
    match deserialized {
        Recognition::ColorMatch { lower, upper } => {
            assert_eq!(lower, vec![0, 0, 0]);
            assert_eq!(upper, vec![255, 255, 255]);
        }
        _ => panic!("Expected ColorMatch recognition"),
    }
}

#[test]
fn test_region_center() {
    let region = Region {
        x: 100,
        y: 200,
        width: 50,
        height: 30,
    };
    assert_eq!(region.center(), (125, 215));
}

#[test]
fn test_match_result_default() {
    let result = MatchResult::default();
    assert!(!result.matched);
    assert_eq!(result.confidence, 0.0);
    assert!(result.box_region.is_none());
}

fn candidate(rank: usize, x: i32, y: i32, confidence: f64) -> MatchCandidate {
    MatchCandidate {
        rank,
        region: Region {
            x,
            y,
            width: 10,
            height: 10,
        },
        confidence,
        text: Some(format!("candidate-{rank}")),
        selected: false,
    }
}

#[test]
fn test_candidate_orders_and_index() {
    let input = vec![
        candidate(0, 50, 50, 0.8),
        candidate(1, 10, 90, 0.9),
        candidate(2, 90, 10, 0.7),
    ];
    let cases = [
        (CandidateOrder::Native, 0),
        (CandidateOrder::TopToBottom, 2),
        (CandidateOrder::BottomToTop, 1),
        (CandidateOrder::LeftToRight, 1),
        (CandidateOrder::RightToLeft, 2),
        (CandidateOrder::ConfidenceDesc, 1),
    ];
    for (order, expected_native_rank) in cases {
        let (selected, sorted, error) =
            select_candidate(input.clone(), Some(&CandidateSelection { order, index: 0 }));
        assert!(error.is_none());
        assert_eq!(
            selected.unwrap().text,
            Some(format!("candidate-{expected_native_rank}"))
        );
        assert_eq!(
            sorted.iter().filter(|candidate| candidate.selected).count(),
            1
        );
    }
}

#[test]
fn test_candidate_index_and_out_of_range() {
    let input = vec![candidate(0, 0, 0, 0.9), candidate(1, 20, 0, 0.8)];
    let (selected, _, error) = select_candidate(
        input.clone(),
        Some(&CandidateSelection {
            order: CandidateOrder::Native,
            index: 1,
        }),
    );
    assert_eq!(selected.unwrap().text.as_deref(), Some("candidate-1"));
    assert!(error.is_none());

    let (selected, candidates, error) = select_candidate(
        input,
        Some(&CandidateSelection {
            order: CandidateOrder::Native,
            index: 2,
        }),
    );
    assert!(selected.is_none());
    assert!(candidates.iter().all(|candidate| !candidate.selected));
    assert!(error.unwrap().contains("2 eligible candidate(s)"));
}

#[test]
fn test_candidate_selection_defaults_and_serde() {
    let defaults: CandidateSelection = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults.order, CandidateOrder::Native);
    assert_eq!(defaults.index, 0);

    let configured: CandidateSelection =
        serde_json::from_str(r#"{"order":"right_to_left","index":3}"#).unwrap();
    assert_eq!(configured.order, CandidateOrder::RightToLeft);
    assert_eq!(configured.index, 3);
}

#[test]
fn test_candidate_order_ties_use_other_axis_then_native_index() {
    let input = vec![
        candidate(0, 20, 10, 0.8),
        candidate(1, 10, 10, 0.8),
        candidate(2, 10, 10, 0.8),
    ];
    let (_, sorted, error) = select_candidate(
        input,
        Some(&CandidateSelection {
            order: CandidateOrder::TopToBottom,
            index: 0,
        }),
    );

    assert!(error.is_none());
    assert_eq!(sorted[0].text.as_deref(), Some("candidate-1"));
    assert_eq!(sorted[1].text.as_deref(), Some("candidate-2"));
    assert_eq!(sorted[2].text.as_deref(), Some("candidate-0"));
}

#[test]
fn effective_match_applies_inverse_but_never_inverts_selection_errors() {
    let unmatched = MatchResult::default();
    assert!(!effective_match(&unmatched, false));
    assert!(effective_match(&unmatched, true));

    let invalid_selection = MatchResult {
        selection_error: Some("candidate index is out of range".to_string()),
        ..MatchResult::default()
    };
    assert!(!effective_match(&invalid_selection, true));
}

#[test]
fn debug_candidate_limit_preserves_a_selected_candidate_outside_the_prefix() {
    let mut candidates = (0..150)
        .map(|rank| {
            let mut value = candidate(rank, rank as i32, 0, 1.0);
            value.selected = rank == 120;
            value
        })
        .collect();
    limit_debug_candidates(&mut candidates, 100);
    assert_eq!(candidates.len(), 100);
    assert!(candidates
        .iter()
        .any(|candidate| candidate.rank == 120 && candidate.selected));
}

#[test]
fn test_recognition_substitute_ocr_expected() {
    let recognition = Recognition::Ocr {
        expected: Some("{{key}}".to_string()),
        threshold: None,
    };
    let mut params = std::collections::HashMap::new();
    params.insert("key".to_string(), serde_json::Value::Number(19.into()));
    let substituted = recognition.substitute(&params);
    match substituted {
        Recognition::Ocr { expected, .. } => assert_eq!(expected.as_deref(), Some("19")),
        _ => panic!("Expected Ocr variant"),
    }
}

#[test]
fn test_recognition_substitute_no_params_returns_clone() {
    let recognition = Recognition::Ocr {
        expected: Some("{{key}}".to_string()),
        threshold: None,
    };
    let params = std::collections::HashMap::new();
    let substituted = recognition.substitute(&params);
    match substituted {
        Recognition::Ocr { expected, .. } => assert_eq!(expected.as_deref(), Some("{{key}}")),
        _ => panic!("Expected Ocr variant"),
    }
}

#[test]
fn test_recognition_substitute_direct_hit_unchanged() {
    let recognition = Recognition::DirectHit;
    let mut params = std::collections::HashMap::new();
    params.insert("key".to_string(), serde_json::Value::Number(19.into()));
    let substituted = recognition.substitute(&params);
    assert!(matches!(substituted, Recognition::DirectHit));
}

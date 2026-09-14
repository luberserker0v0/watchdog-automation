use super::*;
use crate::modules::pipeline::models::recognizer::{Recognition, RecognitionConfig};

#[test]
fn test_node_creation() {
    let node = Node {
        id: "Start".to_string(),
        label: "Start".to_string(),
        config: NodeConfig {
            recognition: RecognitionConfig {
                kind: Recognition::TemplateMatch {
                    template: "start.png".to_string(),
                    threshold: None,
                },
                ..RecognitionConfig::default()
            },
            actions: vec![Action::DoNothing],
            ..NodeConfig::default()
        },
    };
    assert_eq!(node.id, "Start");
    assert!(node.enabled);
    assert_eq!(node.timeout, 20000);
}

#[test]
fn test_node_serde_keeps_recognition_owned_fields_nested() {
    let node = Node {
        id: "Test".to_string(),
        label: "Test".to_string(),
        config: NodeConfig {
            recognition: RecognitionConfig {
                kind: Recognition::Ocr {
                    expected: Some("hello".to_string()),
                    threshold: None,
                },
                roi: Some([10, 20, 100, 50]),
                roi_offset: Some([1, 2, 3, 4]),
                candidate_selection: None,
            },
            timeout: 10000,
            rate_limit: 500,
            pre_delay: 100,
            post_delay: 100,
            max_hit: 10,
            repeat: 2,
            repeat_delay: 50,
            ..NodeConfig::default()
        },
    };
    let value = serde_json::to_value(&node).unwrap();
    assert_eq!(
        value["recognition"]["roi"],
        serde_json::json!([10, 20, 100, 50])
    );
    assert_eq!(
        value["recognition"]["roi_offset"],
        serde_json::json!([1, 2, 3, 4])
    );
    assert!(value.get("roi").is_none());
    assert!(value.get("roi_offset").is_none());
    let deserialized: Node = serde_json::from_value(value).unwrap();
    assert_eq!(deserialized.id, "Test");
    assert_eq!(deserialized.timeout, 10000);
    assert_eq!(deserialized.max_hit, 10);
}

#[test]
fn test_node_defaults() {
    let node: Node = serde_json::from_str(r#"{"id":"Test"}"#).unwrap();
    assert_eq!(node.id, "Test");
    assert!(matches!(node.recognition.kind, Recognition::DirectHit));
    assert!(node.actions.is_empty());
    assert!(node.recognition.roi.is_none());
    assert!(node.recognition.roi_offset.is_none());
    assert!(node.recognition.candidate_selection.is_none());
    assert_eq!(node.timeout, 20000);
    assert_eq!(node.rate_limit, 1000);
    assert_eq!(node.pre_delay, 200);
    assert_eq!(node.post_delay, 200);
    assert!(node.enabled);
    assert!(!node.inverse);
    assert_eq!(node.max_hit, u32::MAX);
    assert_eq!(node.repeat, 1);
    assert_eq!(node.repeat_delay, 0);
}

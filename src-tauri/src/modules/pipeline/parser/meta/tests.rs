use super::parse;
use crate::modules::pipeline::models::recognizer::Recognition;

#[test]
fn parses_the_single_metadata_shape() {
    let metadata = parse(
        r#"{
        "nodes": {
            "Login": {
                "recognition": {
                    "type": "TemplateMatch",
                    "template": "login.png",
                    "threshold": 0.8,
                    "roi": [10, 20, 100, 50]
                },
                "actions": [{ "type": "Click", "target": true }],
                "pre_delay": 500
            }
        }
    }"#,
    )
    .unwrap();

    let node = metadata.nodes.get("Login").unwrap();
    assert_eq!(node.actions.len(), 1);
    assert_eq!(node.pre_delay, 500);
    assert_eq!(node.recognition.roi, Some([10, 20, 100, 50]));
    assert!(matches!(
        node.recognition.kind,
        Recognition::TemplateMatch { .. }
    ));
}

#[test]
fn applies_node_defaults() {
    let metadata = parse(r#"{"nodes":{"Done":{"recognition":{"type":"DirectHit"}}}}"#).unwrap();
    let node = metadata.nodes.get("Done").unwrap();
    assert!(metadata.edges.is_empty());
    assert!(node.actions.is_empty());
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

#[test]
fn rejects_invalid_json() {
    assert!(parse("not valid json").is_err());
}

#[test]
fn rejects_flow_fields_in_metadata_json() {
    assert!(parse(r#"{"version":"1.0","nodes":{},"edges":[]}"#).is_err());
}

#[test]
fn parses_mermaid_edge_configuration() {
    let metadata = parse(r#"{"nodes":{},"edges":{"Login->Error":{"edge_type":"error"}}}"#).unwrap();
    assert!(matches!(
        metadata.edges["Login->Error"].edge_type,
        crate::modules::pipeline::models::edge::EdgeType::Error
    ));
}

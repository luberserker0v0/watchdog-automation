use super::parse;
use crate::modules::pipeline::authoring::models::graph::{CaptureArea, PipelineTarget};
use crate::modules::pipeline::authoring::models::recognizer::Recognition;

#[test]
fn patrol_configuration_is_rejected_from_metadata() {
    let error = parse(r#"{"nodes":{},"patrol":{"mode":"record"}}"#).unwrap_err();
    assert!(error.contains("/patrol"), "{error}");
    assert!(error.contains("Settings"), "{error}");
}

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
    assert_eq!(
        node.recognition.roi,
        Some(
            crate::modules::pipeline::authoring::models::recognizer::RecognitionRoi::Fixed([
                10, 20, 100, 50
            ])
        )
    );
    assert!(matches!(
        node.recognition.kind,
        Recognition::TemplateMatch { .. }
    ));
}

#[test]
fn parses_a_prior_node_recognition_roi_without_changing_fixed_roi_format() {
    let metadata = parse(r#"{"nodes":{"FindPanel":{},"FindButton":{"recognition":{"type":"OCR","roi":"FindPanel","roi_offset":[2,3,4,5]}}}}"#).unwrap();
    let roi = metadata
        .nodes
        .get("FindButton")
        .unwrap()
        .recognition
        .roi
        .as_ref();
    assert_eq!(
        roi,
        Some(
            &crate::modules::pipeline::authoring::models::recognizer::RecognitionRoi::Node(
                "FindPanel".into()
            )
        )
    );
}

#[test]
fn applies_node_defaults() {
    let metadata = parse(r#"{"nodes":{"Done":{}}}"#).unwrap();
    let node = metadata.nodes.get("Done").unwrap();
    assert!(matches!(node.recognition.kind, Recognition::DirectHit));
    assert!(node.actions.is_empty());
    assert_eq!(node.timeout, 20000);
    assert_eq!(node.error_timeout, 20000);
    assert_eq!(node.desc, "");
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
fn parses_entry_kind_and_error_timeout_override() {
    let metadata =
        parse(r#"{"nodes":{"Start":{"kind":"Entry","timeout":5000,"error_timeout":7000}}}"#)
            .unwrap();
    let node = metadata.nodes.get("Start").unwrap();
    assert!(matches!(
        node.kind,
        crate::modules::pipeline::authoring::models::node::NodeKind::Entry
    ));
    assert_eq!(node.timeout, 5000);
    assert_eq!(node.error_timeout, 7000);
}

#[test]
fn rejects_invalid_json() {
    assert!(parse("not valid json").is_err());
}

#[test]
fn rejects_schema_threshold_outside_unit_interval() {
    let error = parse(r#"{"nodes":{"A":{"recognition":{"type":"TemplateMatch","template":"a.png","threshold":1.5}}}}"#).unwrap_err();
    assert!(error.contains("threshold"), "{error}");
}

#[test]
fn rejects_unknown_fields_in_metadata_json() {
    assert!(parse(r#"{"nodes":{},"edges":{}}"#).is_err());
}

#[test]
fn rejects_visual_layout_fields_in_metadata_json() {
    assert!(parse(r#"{"nodes":{},"layout":{"nodes":{}}}"#).is_err());
}

#[test]
fn parses_top_level_filterless_window_target() {
    let metadata = parse(
        r#"{
            "target": {"type":"window","capture_area":"client"},
            "nodes": {}
        }"#,
    )
    .unwrap();
    assert!(matches!(
        metadata.target,
        Some(PipelineTarget::Window {
            process: None,
            class: None,
            title: None,
            capture_area: CaptureArea::Client,
            window_size: None,
        })
    ));
}

#[test]
fn parses_freeze_waits_and_instance_override() {
    let metadata = parse(r#"{"nodes":{"A":{"pre_wait_freezes":500,"post_wait_freezes":{"time":400,"threshold":0.97,"timeout":2000,"roi":[1,2,30,40]}}},"instances":{"B":{"node":"A","pre_wait_freezes":0}}}"#).unwrap();
    assert_eq!(
        metadata.nodes["A"]
            .pre_wait_freezes
            .settings()
            .unwrap()
            .unwrap()
            .0,
        500
    );
    assert_eq!(
        metadata.nodes["A"]
            .post_wait_freezes
            .settings()
            .unwrap()
            .unwrap()
            .1,
        0.97
    );
    assert!(metadata.instances.unwrap()["B"]
        .pre_wait_freezes
        .as_ref()
        .unwrap()
        .settings()
        .unwrap()
        .is_none());
}

#[test]
fn rejects_invalid_freeze_wait_with_field_diagnostic() {
    let error =
        parse(r#"{"nodes":{"A":{"pre_wait_freezes":{"time":500,"timeout":100}}}}"#).unwrap_err();
    assert!(error.contains("/nodes/A/pre_wait_freezes"), "{error}");
}

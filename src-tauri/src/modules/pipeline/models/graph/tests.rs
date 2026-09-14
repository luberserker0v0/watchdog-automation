use super::*;
use crate::modules::pipeline::models::edge::EdgeType;
use crate::modules::pipeline::models::node::NodeConfig;

fn node(id: &str) -> Node {
    Node {
        id: id.to_string(),
        label: id.to_string(),
        config: NodeConfig::default(),
    }
}

#[test]
fn test_graph_creation() {
    let graph = PipelineGraph {
        version: "1.0".to_string(),
        desktop: None,
        target: None,
        nodes: vec![node("A")],
        edges: vec![Edge {
            from: "A".to_string(),
            to: "A".to_string(),
            edge_type: EdgeType::Next,
        }],
    };
    assert_eq!(graph.version, "1.0");
    assert_eq!(graph.nodes.len(), 1);
}

#[test]
fn test_graph_serde() {
    let graph = PipelineGraph {
        version: "1.0".to_string(),
        desktop: None,
        target: None,
        nodes: vec![],
        edges: vec![],
    };
    let json = serde_json::to_string(&graph).unwrap();
    assert!(json.contains("\"version\":\"1.0\""));
    let deserialized: PipelineGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.version, "1.0");
}

#[test]
fn test_graph_find_node() {
    let graph = PipelineGraph {
        version: "1.0".to_string(),
        desktop: None,
        target: None,
        nodes: vec![node("A")],
        edges: vec![],
    };
    assert!(graph.find_node("A").is_some());
    assert!(graph.find_node("B").is_none());
}

#[test]
fn test_graph_next_edges() {
    let graph = PipelineGraph {
        version: "1.0".to_string(),
        desktop: None,
        target: None,
        nodes: vec![node("A"), node("B")],
        edges: vec![
            Edge {
                from: "A".to_string(),
                to: "B".to_string(),
                edge_type: EdgeType::Next,
            },
            Edge {
                from: "A".to_string(),
                to: "A".to_string(),
                edge_type: EdgeType::Error,
            },
        ],
    };
    let next = graph.next_edges("A");
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].to, "B");
    let error = graph.error_edges("A");
    assert_eq!(error.len(), 1);
    assert_eq!(error[0].to, "A");
}

#[test]
fn test_graph_entry_node() {
    let graph = PipelineGraph {
        version: "1.0".to_string(),
        desktop: None,
        target: None,
        nodes: vec![node("A"), node("B")],
        edges: vec![Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            edge_type: EdgeType::Next,
        }],
    };
    let entry = graph.entry_node();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().id, "A");
}

#[test]
fn test_window_capture_area_defaults_to_frame() {
    let target: PipelineTarget =
        serde_json::from_str(r#"{"type":"window","title":"Example"}"#).unwrap();
    assert!(matches!(
        target,
        PipelineTarget::Window {
            capture_area: CaptureArea::Frame,
            ..
        }
    ));
    let client: PipelineTarget =
        serde_json::from_str(r#"{"type":"window","title":"Example","capture_area":"client"}"#)
            .unwrap();
    assert!(matches!(
        client,
        PipelineTarget::Window {
            capture_area: CaptureArea::Client,
            ..
        }
    ));
}

#[test]
fn filterless_window_target_is_valid_and_inherits_window_identity() {
    let target: PipelineTarget =
        serde_json::from_str(r#"{"type":"window","capture_area":"client"}"#).unwrap();
    assert!(!target.has_window_filters());
    assert!(matches!(
        target,
        PipelineTarget::Window {
            process: None,
            class: None,
            title: None,
            capture_area: CaptureArea::Client,
            window_size: None,
        }
    ));

    let filtered: PipelineTarget =
        serde_json::from_str(r#"{"type":"window","process":"watch-dog-target"}"#).unwrap();
    assert!(filtered.has_window_filters());
}

#[test]
fn window_size_is_client_size_metadata_independent_of_capture_area() {
    let target: PipelineTarget = serde_json::from_str(
        r#"{"type":"window","capture_area":"frame","window_size":[1280,720]}"#,
    )
    .unwrap();
    assert_eq!(target.window_size(), Some([1280, 720]));
    assert!(matches!(
        target,
        PipelineTarget::Window {
            capture_area: CaptureArea::Frame,
            window_size: Some([1280, 720]),
            ..
        }
    ));
}

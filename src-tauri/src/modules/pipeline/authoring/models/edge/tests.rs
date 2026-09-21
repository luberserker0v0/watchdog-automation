use super::*;
use serde_json;

#[test]
fn test_edge_creation() {
    let edge = Edge {
        from: "A".to_string(),
        to: "B".to_string(),
        edge_type: EdgeType::Next,
    };

    assert_eq!(edge.from, "A");
    assert_eq!(edge.to, "B");
    assert_eq!(edge.edge_type, EdgeType::Next);
}

#[test]
fn test_edge_error_type() {
    let edge = Edge {
        from: "A".to_string(),
        to: "B".to_string(),
        edge_type: EdgeType::Error,
    };

    assert_eq!(edge.edge_type, EdgeType::Error);
}

#[test]
fn test_edge_serde() {
    let edge = Edge {
        from: "A".to_string(),
        to: "B".to_string(),
        edge_type: EdgeType::Next,
    };

    let json = serde_json::to_string(&edge).unwrap();
    let deserialized: Edge = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.from, "A");
    assert_eq!(deserialized.edge_type, EdgeType::Next);
}

#[test]
fn test_edge_error_serde() {
    let edge = Edge {
        from: "A".to_string(),
        to: "B".to_string(),
        edge_type: EdgeType::Error,
    };

    let json = serde_json::to_string(&edge).unwrap();
    assert!(json.contains("\"edge_type\":\"error\""));

    let deserialized: Edge = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.edge_type, EdgeType::Error);
}

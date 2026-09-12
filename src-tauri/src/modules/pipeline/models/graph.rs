use super::edge::Edge;
use super::node::Node;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureArea {
    #[default]
    Frame,
    Client,
}

impl CaptureArea {
    pub fn is_frame(value: &Self) -> bool {
        *value == Self::Frame
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PipelineTarget {
    Screen {
        #[serde(default)]
        index: usize,
    },
    Window {
        #[serde(skip_serializing_if = "Option::is_none")]
        process: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        class: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "CaptureArea::is_frame")]
        capture_area: CaptureArea,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineGraph {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<PipelineTarget>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl PipelineGraph {
    pub fn find_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn next_edges(&self, from_id: &str) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|e| e.from == from_id && e.edge_type == super::edge::EdgeType::Next)
            .collect()
    }

    pub fn error_edges(&self, from_id: &str) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|e| e.from == from_id && e.edge_type == super::edge::EdgeType::Error)
            .collect()
    }

    pub fn entry_node(&self) -> Option<&Node> {
        let has_incoming: std::collections::HashSet<&str> =
            self.edges.iter().map(|e| e.to.as_str()).collect();
        self.nodes
            .iter()
            .find(|n| !has_incoming.contains(n.id.as_str()))
    }
}

#[cfg(test)]
mod tests;

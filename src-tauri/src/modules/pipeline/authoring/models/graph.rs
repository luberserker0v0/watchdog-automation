use super::edge::Edge;
use super::node::Node;
pub use crate::modules::capture::models::target::{CaptureArea, PipelineTarget};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaunchConfig {
    /// Path to executable to run
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    /// Command to run (alternative to exe)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
    /// Arguments for the executable
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Run command in shell
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shell: bool,
    /// Working directory for the process
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Environment variables for the process
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub env: std::collections::HashMap<String, String>,
    /// Wait for a window matching the pipeline target to appear
    #[serde(default)]
    pub wait_for_window: bool,
    /// Timeout in milliseconds to wait for window (default: 10000)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineGraph {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<PipelineTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch: Option<LaunchConfig>,
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

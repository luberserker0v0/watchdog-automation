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
        /// Expected client/content dimensions in physical capture pixels.
        #[serde(skip_serializing_if = "Option::is_none")]
        window_size: Option<[u32; 2]>,
    },
}

impl PipelineTarget {
    pub fn has_window_filters(&self) -> bool {
        matches!(
            self,
            Self::Window { process, class, title, .. }
                if process.is_some() || class.is_some() || title.is_some()
        )
    }

    pub fn window_size(&self) -> Option<[u32; 2]> {
        match self {
            Self::Window { window_size, .. } => *window_size,
            Self::Screen { .. } => None,
        }
    }
}

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
    pub desktop: Option<String>,
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

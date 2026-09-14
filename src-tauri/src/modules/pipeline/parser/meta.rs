use crate::modules::pipeline::models::edge::EdgeType;
use crate::modules::pipeline::models::graph::PipelineTarget;
use crate::modules::pipeline::models::node::NodeConfig;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataFile {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<PipelineTarget>,
    #[serde(default)]
    pub nodes: HashMap<String, NodeConfig>,
    #[serde(default)]
    pub edges: HashMap<String, EdgeConfig>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeConfig {
    #[serde(default)]
    pub edge_type: EdgeType,
}

pub fn parse(content: &str) -> Result<MetadataFile, String> {
    serde_json::from_str(content).map_err(|error| format!("Invalid node metadata JSON: {}", error))
}

pub fn to_json(metadata: &MetadataFile) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(metadata)
}

#[cfg(test)]
mod tests;

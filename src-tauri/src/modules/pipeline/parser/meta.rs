use crate::modules::pipeline::models::graph::{LaunchConfig, PipelineTarget};
use crate::modules::pipeline::models::node::{NodeConfig, NodeKind};
use crate::modules::pipeline::models::params::ParamDef;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InstanceConfig {
    /// Reference to a node template in "nodes"
    pub node: String,
    /// Per-instance parameter definitions (for UI)
    #[serde(default)]
    pub params: Vec<ParamDef>,
    /// Hardcoded parameter values for template substitution
    #[serde(default)]
    pub param_values: HashMap<String, serde_json::Value>,
    /// Structural node kind override (`Normal` default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<NodeKind>,
    /// Override fields from node template
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_delay: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_delay: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_delay: Option<u64>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataFile {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<PipelineTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<LaunchConfig>,
    #[serde(default)]
    pub nodes: HashMap<String, NodeConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instances: Option<HashMap<String, InstanceConfig>>,
}

pub fn parse(content: &str) -> Result<MetadataFile, String> {
    serde_json::from_str(content).map_err(|error| format!("Invalid node metadata JSON: {}", error))
}

pub fn to_json(metadata: &MetadataFile) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(metadata)
}

#[cfg(test)]
mod tests;

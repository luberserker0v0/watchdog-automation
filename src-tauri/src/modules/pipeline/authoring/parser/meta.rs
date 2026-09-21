use crate::modules::pipeline::authoring::models::graph::{LaunchConfig, PipelineTarget};
use crate::modules::pipeline::authoring::models::node::{FreezeWait, NodeConfig, NodeKind};
use crate::modules::pipeline::authoring::models::params::ParamDef;
use std::collections::HashMap;
use std::sync::OnceLock;

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
    /// Free-text description override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
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
    pub pre_wait_freezes: Option<FreezeWait>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_delay: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_wait_freezes: Option<FreezeWait>,
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
    static VALIDATOR: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();
    let validator = VALIDATOR
        .get_or_init(|| {
            let schema: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../../schemas/watchdog-metadata.schema.json"
            ))
            .map_err(|error| error.to_string())?;
            jsonschema::draft202012::new(&schema).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| format!("Invalid bundled metadata schema: {error}"))?;
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| format!("Invalid node metadata JSON: {error}"))?;
    if value.get("patrol").is_some() {
        return Err("Invalid node metadata at /patrol: patrol is configured only in Settings; remove the patrol field from .meta.json".into());
    }
    if let Some(error) = validator.iter_errors(&value).next() {
        return Err(format!(
            "Invalid node metadata at {}: {}",
            error.instance_path(),
            error
        ));
    }
    let metadata: MetadataFile = serde_json::from_value(value)
        .map_err(|error| format!("Invalid node metadata JSON: {error}"))?;
    for (id, node) in &metadata.nodes {
        node.pre_wait_freezes.settings().map_err(|error| {
            format!("Invalid node metadata at /nodes/{id}/pre_wait_freezes: {error}")
        })?;
        node.post_wait_freezes.settings().map_err(|error| {
            format!("Invalid node metadata at /nodes/{id}/post_wait_freezes: {error}")
        })?;
    }
    if let Some(instances) = &metadata.instances {
        for (id, instance) in instances {
            if let Some(wait) = &instance.pre_wait_freezes {
                wait.settings().map_err(|error| {
                    format!("Invalid node metadata at /instances/{id}/pre_wait_freezes: {error}")
                })?;
            }
            if let Some(wait) = &instance.post_wait_freezes {
                wait.settings().map_err(|error| {
                    format!("Invalid node metadata at /instances/{id}/post_wait_freezes: {error}")
                })?;
            }
        }
    }
    Ok(metadata)
}

pub fn to_json(metadata: &MetadataFile) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(metadata)
}

#[cfg(test)]
mod tests;

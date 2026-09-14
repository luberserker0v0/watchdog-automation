use super::action::Action;
use super::defaults::*;
use super::graph::PipelineTarget;
use super::params::ParamDef;
use super::recognizer::RecognitionConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(default = "default_recognition")]
    pub recognition: RecognitionConfig,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub params: Vec<ParamDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<PipelineTarget>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u64,
    #[serde(default = "default_pre_delay")]
    pub pre_delay: u64,
    #[serde(default = "default_post_delay")]
    pub post_delay: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub inverse: bool,
    #[serde(default = "default_max_hit")]
    pub max_hit: u32,
    #[serde(default = "default_repeat")]
    pub repeat: u32,
    #[serde(default)]
    pub repeat_delay: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attach: Option<serde_json::Value>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            recognition: default_recognition(),
            actions: Vec::new(),
            params: Vec::new(),
            target: None,
            timeout: default_timeout(),
            rate_limit: default_rate_limit(),
            pre_delay: default_pre_delay(),
            post_delay: default_post_delay(),
            enabled: default_true(),
            inverse: false,
            max_hit: default_max_hit(),
            repeat: default_repeat(),
            repeat_delay: 0,
            attach: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(flatten)]
    pub config: NodeConfig,
}

impl std::ops::Deref for Node {
    type Target = NodeConfig;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl std::ops::DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

#[cfg(test)]
mod tests;

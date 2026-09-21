use super::action::Action;
use super::defaults::*;
use super::graph::PipelineTarget;
use super::params::ParamDef;
use super::recognizer::RecognitionConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FreezeWait {
    Milliseconds(u64),
    Options {
        time: u64,
        #[serde(default = "default_freeze_threshold")]
        threshold: f64,
        #[serde(default = "default_freeze_rate_limit")]
        rate_limit: u64,
        #[serde(default = "default_freeze_timeout")]
        timeout: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        roi: Option<[u32; 4]>,
    },
}

fn default_freeze_threshold() -> f64 {
    0.98
}
fn default_freeze_rate_limit() -> u64 {
    200
}
fn default_freeze_timeout() -> u64 {
    10_000
}

impl Default for FreezeWait {
    fn default() -> Self {
        Self::Milliseconds(0)
    }
}

impl FreezeWait {
    pub fn settings(&self) -> Result<Option<(u64, f64, u64, u64, Option<[u32; 4]>)>, String> {
        let (time, threshold, rate_limit, timeout, roi) = match self {
            Self::Milliseconds(time) => (
                *time,
                default_freeze_threshold(),
                default_freeze_rate_limit(),
                default_freeze_timeout().max(time.saturating_add(5_000)),
                None,
            ),
            Self::Options {
                time,
                threshold,
                rate_limit,
                timeout,
                roi,
            } => (*time, *threshold, *rate_limit, *timeout, *roi),
        };
        if time == 0 {
            return Ok(None);
        }
        if time > 30_000
            || timeout == 0
            || timeout > 120_000
            || time > timeout
            || rate_limit == 0
            || rate_limit > 5_000
            || !threshold.is_finite()
            || !(0.0..=1.0).contains(&threshold)
        {
            return Err("Invalid freeze wait: require time 1..30000 ms, timeout >= time and <=120000 ms, rate_limit 1..5000 ms, threshold 0..1".into());
        }
        if let Some([_, _, width, height]) = roi {
            if width == 0 || height == 0 {
                return Err("Freeze wait ROI must have positive width and height".into());
            }
        }
        Ok(Some((time, threshold, rate_limit, timeout, roi)))
    }
}

/// Structural node type. `Entry` starts a run; `Endpoint` completes it.
/// Neither performs recognition or actions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    #[default]
    Normal,
    Entry,
    Endpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(default)]
    pub kind: NodeKind,
    /// Free-text description of what this node does. Display-only.
    #[serde(default)]
    pub desc: String,
    #[serde(default = "default_recognition")]
    pub recognition: RecognitionConfig,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<PipelineTarget>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Quota for executing the error target after the children quota expires.
    #[serde(default = "default_timeout")]
    pub error_timeout: u64,
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u64,
    #[serde(default = "default_pre_delay")]
    pub pre_delay: u64,
    #[serde(default)]
    pub pre_wait_freezes: FreezeWait,
    #[serde(default = "default_post_delay")]
    pub post_delay: u64,
    #[serde(default)]
    pub post_wait_freezes: FreezeWait,
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
            kind: NodeKind::Normal,
            desc: String::new(),
            recognition: default_recognition(),
            actions: Vec::new(),
            target: None,
            timeout: default_timeout(),
            error_timeout: default_timeout(),
            rate_limit: default_rate_limit(),
            pre_delay: default_pre_delay(),
            pre_wait_freezes: FreezeWait::default(),
            post_delay: default_post_delay(),
            post_wait_freezes: FreezeWait::default(),
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamDef>,
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

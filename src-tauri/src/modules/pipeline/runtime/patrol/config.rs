#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatrolConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_k_hops")]
    pub k_hops: usize,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    #[serde(default)]
    pub action_effect_threshold: Option<f64>,
    #[serde(default)]
    pub redirect_threshold: Option<f64>,
    #[serde(default = "default_min_margin")]
    pub min_margin: f64,
    #[serde(default = "default_cycle_seconds")]
    pub cycle_seconds: u64,
    #[serde(default)]
    pub recovery_policy: RecoveryPolicy,
    #[serde(default = "default_max_redirects")]
    pub max_redirects: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPolicy {
    #[default]
    HintOnly,
    RedirectionOnly,
}

impl Default for PatrolConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            k_hops: default_k_hops(),
            threshold: default_threshold(),
            action_effect_threshold: None,
            redirect_threshold: None,
            min_margin: default_min_margin(),
            cycle_seconds: default_cycle_seconds(),
            recovery_policy: RecoveryPolicy::HintOnly,
            max_redirects: default_max_redirects(),
        }
    }
}

fn default_k_hops() -> usize {
    1
}
fn default_threshold() -> f64 {
    0.85
}
fn default_min_margin() -> f64 {
    0.1
}
fn default_cycle_seconds() -> u64 {
    20
}
fn default_max_redirects() -> u32 {
    1
}

impl PatrolConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=4).contains(&self.k_hops) {
            return Err("Settings patrol K hops must be between 1 and 4".into());
        }
        if !self.threshold.is_finite() || !(0.0..=1.0).contains(&self.threshold) {
            return Err("Settings patrol threshold must be between 0 and 1".into());
        }
        for (label, value) in [
            ("action-effect", self.action_effect_threshold),
            ("redirect", self.redirect_threshold),
        ] {
            if value.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
                return Err(format!(
                    "Settings patrol {label} threshold must be between 0 and 1"
                ));
            }
        }
        if !self.min_margin.is_finite() || !(0.0..=1.0).contains(&self.min_margin) {
            return Err("Settings patrol minimum margin must be between 0 and 1".into());
        }
        if !(1..=3600).contains(&self.cycle_seconds) {
            return Err("Settings patrol cycle must be between 1 and 3600 seconds".into());
        }
        if !(1..=10).contains(&self.max_redirects) {
            return Err("Settings patrol maximum redirects must be between 1 and 10".into());
        }
        Ok(())
    }

    pub fn action_effect_threshold(&self) -> f64 {
        self.action_effect_threshold.unwrap_or(self.threshold)
    }

    pub fn redirect_threshold(&self) -> f64 {
        self.redirect_threshold.unwrap_or(self.threshold)
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct VlmConnection {
    pub endpoint: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

impl VlmConnection {
    pub fn validate(&self) -> Result<(), String> {
        let url = reqwest::Url::parse(&self.endpoint)
            .map_err(|error| format!("Invalid LLM endpoint URL: {error}"))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("LLM endpoint must be an HTTP or HTTPS URL".into());
        }
        if self.model.trim().is_empty() {
            return Err("LLM model name is required when patrol is enabled".into());
        }
        Ok(())
    }
}

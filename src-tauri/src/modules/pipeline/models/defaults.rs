use super::constants::*;
use super::recognizer::RecognitionConfig;

pub fn default_recognition() -> RecognitionConfig {
    RecognitionConfig::default()
}

pub fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

pub fn default_rate_limit() -> u64 {
    DEFAULT_RATE_LIMIT_MS
}

pub fn default_pre_delay() -> u64 {
    DEFAULT_PRE_DELAY_MS
}

pub fn default_post_delay() -> u64 {
    DEFAULT_POST_DELAY_MS
}

pub fn default_true() -> bool {
    true
}

pub fn default_max_hit() -> u32 {
    DEFAULT_MAX_HIT
}

pub fn default_repeat() -> u32 {
    DEFAULT_REPEAT
}

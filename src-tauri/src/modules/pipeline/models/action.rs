use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "DoNothing")]
    DoNothing,

    #[serde(rename = "Click")]
    Click {
        #[serde(default = "default_true")]
        target: Target,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_offset: Option<[i32; 4]>,
    },

    #[serde(rename = "LongPress")]
    LongPress {
        #[serde(default = "default_true")]
        target: Target,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_offset: Option<[i32; 4]>,
        #[serde(default = "default_long_press_duration")]
        duration: u64,
    },

    #[serde(rename = "Swipe")]
    Swipe {
        #[serde(default = "default_true")]
        begin: Target,
        #[serde(skip_serializing_if = "Option::is_none")]
        begin_offset: Option<[i32; 4]>,
        #[serde(default = "default_true")]
        end: Target,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_offset: Option<[i32; 4]>,
        #[serde(default = "default_swipe_duration")]
        duration: u64,
    },

    #[serde(rename = "Scroll")]
    Scroll {
        #[serde(default = "default_true")]
        target: Target,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_offset: Option<[i32; 4]>,
        #[serde(default)]
        dx: i32,
        #[serde(default)]
        dy: i32,
    },

    #[serde(rename = "ClickKey")]
    ClickKey { key: Vec<u32> },

    #[serde(rename = "InputText")]
    InputText { input_text: String },

    #[serde(rename = "StopTask")]
    StopTask,

    #[serde(rename = "Custom")]
    Custom {
        action: String,
        #[serde(default)]
        param: serde_json::Value,
    },
}

fn default_true() -> Target {
    Target::recognized()
}

fn default_long_press_duration() -> u64 {
    1000
}

fn default_swipe_duration() -> u64 {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Target {
    RecognizedBool(bool),
    Coordinate([i32; 2]),
    Region([i32; 4]),
    Node(String),
}

impl Target {
    pub fn recognized() -> Self {
        Target::RecognizedBool(true)
    }
}

impl Default for Target {
    fn default() -> Self {
        Target::RecognizedBool(true)
    }
}

#[cfg(test)]
mod tests;

use crate::modules::capture::models::target::CaptureArea;
use crate::modules::pipeline::authoring::models::action::Action;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RecordingTarget {
    Screen {
        index: usize,
        label: String,
    },
    Window {
        hwnd: usize,
        title: String,
        capture_area: CaptureArea,
        frame_width: u32,
        frame_height: u32,
    },
}

impl RecordingTarget {
    pub fn label(&self) -> String {
        match self {
            Self::Screen { label, .. } => label.clone(),
            Self::Window { title, .. } => title.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecordingConfig {
    pub target: RecordingTarget,
    #[serde(default)]
    pub capture_text: bool,
    #[serde(default)]
    pub excluded_key_codes: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecorderState {
    Idle,
    Recording,
    Paused,
    Stopping,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordedActionDraft {
    pub id: u64,
    pub offset_ms: u64,
    pub action: Action,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordingSession {
    pub id: String,
    pub started_unix_ms: u64,
    pub duration_ms: u64,
    pub target: RecordingTarget,
    pub actions: Vec<RecordedActionDraft>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecorderStatus {
    pub state: RecorderState,
    pub session_id: Option<String>,
    pub elapsed_ms: u64,
    pub event_count: usize,
    pub target_label: Option<String>,
    pub reason: Option<String>,
    pub supported: bool,
}

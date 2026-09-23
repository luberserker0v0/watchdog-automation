use std::time::Instant;

use crate::modules::capture::models::target::CaptureArea;
use crate::modules::capture::services::bounds::CaptureBounds;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureEngine {
    Gdi,
    #[cfg(target_os = "windows")]
    Dxgi,
    #[default]
    Graphics,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StreamTarget {
    Screen {
        index: usize,
    },
    Window {
        hwnd: usize,
        #[serde(default)]
        capture_area: CaptureArea,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamFrame {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub target: StreamTarget,
    pub bounds: Option<CaptureBounds>,
    #[serde(skip)]
    pub captured_at: Instant,
    pub session_id: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamStatus {
    pub is_streaming: bool,
    pub fps: u32,
    pub target_type: String,
    pub target_index: usize,
    pub start_time: Option<u64>,
    pub engine: String,
    pub capture_area: String,
    pub owner: String,
    pub owner_id: Option<String>,
    pub session_id: u64,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
    pub client_width: Option<u32>,
    pub client_height: Option<u32>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureArea {
    #[default]
    Frame,
    Client,
}

impl CaptureArea {
    pub fn is_frame(value: &Self) -> bool {
        *value == Self::Frame
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PipelineTarget {
    Screen {
        #[serde(default)]
        index: usize,
    },
    Window {
        #[serde(skip_serializing_if = "Option::is_none")]
        process: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        class: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "CaptureArea::is_frame")]
        capture_area: CaptureArea,
        /// Expected client/content dimensions in physical capture pixels.
        #[serde(skip_serializing_if = "Option::is_none")]
        window_size: Option<[u32; 2]>,
    },
}

impl PipelineTarget {
    pub fn has_window_filters(&self) -> bool {
        matches!(
            self,
            Self::Window { process, class, title, .. }
                if process.is_some() || class.is_some() || title.is_some()
        )
    }

    pub fn window_size(&self) -> Option<[u32; 2]> {
        match self {
            Self::Window { window_size, .. } => *window_size,
            Self::Screen { .. } => None,
        }
    }
}

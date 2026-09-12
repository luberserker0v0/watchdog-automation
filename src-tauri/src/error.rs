use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum CaptureError {
    ScreenNotFound,
    WindowNotFound,
    CaptureFailed(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScreenNotFound => write!(f, "Screen not found"),
            Self::WindowNotFound => write!(f, "Window not found"),
            Self::CaptureFailed(msg) => write!(f, "Capture failed: {}", msg),
        }
    }
}

impl std::error::Error for CaptureError {}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowState {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub index: usize,
    pub title: String,
    pub app_name: String,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hwnd: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WindowDetail {
    pub hwnd: usize,
    pub title: String,
    pub class_name: String,
    pub process_name: String,
    pub width: i32,
    pub height: i32,
}

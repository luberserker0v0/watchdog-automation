use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopInfo {
    pub id: String,
    pub name: Option<String>,
    pub index: u32,
    pub is_current: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum VDError {
    #[error("COM error: {0}")]
    ComError(String),
    #[error("Not initialized")]
    NotInitialized,
}

pub trait VirtualDesktopManager: Send + Sync {
    fn get_all_desktops(&self) -> Result<Vec<DesktopInfo>, VDError>;
    fn get_current_desktop(&self) -> Result<DesktopInfo, VDError>;
    fn get_current_desktop_id(&self) -> Result<String, VDError>;
    fn get_window_desktop_id(&self, hwnd: usize) -> Result<Option<String>, VDError>;
    fn is_window_on_current_desktop(&self, hwnd: usize) -> Result<bool, VDError>;
    fn switch_to_desktop(&self, desktop_id: &str) -> Result<(), VDError>;
}

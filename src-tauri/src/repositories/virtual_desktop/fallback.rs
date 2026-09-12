use super::traits::{DesktopInfo, VDError, VirtualDesktopManager};

pub struct FallbackDesktopManager;

impl VirtualDesktopManager for FallbackDesktopManager {
    fn get_all_desktops(&self) -> Result<Vec<DesktopInfo>, VDError> {
        Ok(vec![DesktopInfo {
            id: "default".into(),
            name: Some("Desktop".into()),
            index: 0,
            is_current: true,
        }])
    }

    fn get_current_desktop(&self) -> Result<DesktopInfo, VDError> {
        Ok(DesktopInfo {
            id: "default".into(),
            name: Some("Desktop".into()),
            index: 0,
            is_current: true,
        })
    }

    fn get_current_desktop_id(&self) -> Result<String, VDError> {
        Ok("default".into())
    }

    fn get_window_desktop_id(&self, _hwnd: usize) -> Result<Option<String>, VDError> {
        Ok(Some("default".into()))
    }

    fn is_window_on_current_desktop(&self, _hwnd: usize) -> Result<bool, VDError> {
        Ok(true)
    }

    fn switch_to_desktop(&self, _desktop_id: &str) -> Result<(), VDError> {
        log::info!("[vd] switch_to_desktop called on fallback (no-op)");
        Ok(())
    }
}

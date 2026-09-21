pub mod fallback;
pub mod traits;

#[cfg(target_os = "windows")]
pub mod windows;

use std::sync::OnceLock;
use traits::VirtualDesktopManager;

pub fn create_desktop_manager() -> Box<dyn VirtualDesktopManager> {
    #[cfg(target_os = "windows")]
    {
        if let Some(mgr) = windows::WindowsDesktopManager::new() {
            return Box::new(mgr);
        }
    }
    Box::new(fallback::FallbackDesktopManager)
}

static DESKTOP_MANAGER: OnceLock<Box<dyn VirtualDesktopManager>> = OnceLock::new();

pub fn get_desktop_manager() -> &'static Box<dyn VirtualDesktopManager> {
    DESKTOP_MANAGER.get_or_init(|| {
        log::info!("[vd] initializing desktop manager via OnceLock");
        let mgr = create_desktop_manager();
        log::info!("[vd] desktop manager initialized");
        mgr
    })
}

use crate::repositories::virtual_desktop::get_desktop_manager;
use crate::repositories::virtual_desktop::traits::DesktopInfo;

#[tauri::command]
pub fn get_desktops() -> Result<Vec<DesktopInfo>, String> {
    log::info!("[desktop] get_desktops called");
    let manager = get_desktop_manager();
    let result = manager.get_all_desktops().map_err(|e| {
        log::error!("[desktop] get_all_desktops failed: {}", e);
        e.to_string()
    });
    match &result {
        Ok(desktops) => log::info!("[desktop] got {} desktops: {:?}", desktops.len(), desktops),
        Err(e) => log::error!("[desktop] error: {}", e),
    }
    result
}

#[tauri::command]
pub fn get_current_desktop() -> Result<DesktopInfo, String> {
    log::info!("[desktop] get_current_desktop called");
    let manager = get_desktop_manager();
    let result = manager.get_current_desktop().map_err(|e| {
        log::error!("[desktop] get_current_desktop failed: {}", e);
        e.to_string()
    });
    log::info!("[desktop] get_current_desktop result: {:?}", result);
    result
}

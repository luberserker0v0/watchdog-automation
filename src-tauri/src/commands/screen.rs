use crate::models::screen::ScreenInfo;
use crate::services;
use crate::utils::paths::visions_dir;

#[tauri::command]
pub fn get_screens() -> Result<Vec<ScreenInfo>, String> {
    services::capture::get_screens().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn capture_screen(index: usize) -> Result<Vec<u8>, String> {
    services::capture::capture_screen(index).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_capture(buf: Vec<u8>, filename: String) -> Result<String, String> {
    let dir = visions_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&filename);
    std::fs::write(&path, &buf).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

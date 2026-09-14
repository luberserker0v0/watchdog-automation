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

#[tauri::command]
pub fn save_region_capture(
    buf: Option<Vec<u8>>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    filename: String,
) -> Result<String, String> {
    if width == 0 || height == 0 {
        return Err("The selected region must have a positive width and height".to_string());
    }
    let bytes = match buf {
        Some(bytes) if !bytes.is_empty() => bytes,
        _ => crate::FRAME_CACHE
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .map(|frame| frame.bytes.clone())
            .ok_or_else(|| "No current capture frame is available".to_string())?,
    };
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("Could not decode the current capture frame: {error}"))?;
    let right = x
        .checked_add(width)
        .ok_or_else(|| "The selected region exceeds the image bounds".to_string())?;
    let bottom = y
        .checked_add(height)
        .ok_or_else(|| "The selected region exceeds the image bounds".to_string())?;
    if right > image.width() || bottom > image.height() {
        return Err(format!(
            "Selected region ({x}, {y}, {width}, {height}) exceeds image size {}x{}",
            image.width(),
            image.height()
        ));
    }

    let safe_filename = std::path::Path::new(&filename)
        .file_name()
        .ok_or_else(|| "A valid region capture filename is required".to_string())?;
    let dir = visions_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join(safe_filename);
    image
        .crop_imm(x, y, width, height)
        .save(&path)
        .map_err(|error| format!("Could not save region capture: {error}"))?;
    Ok(path.to_string_lossy().to_string())
}

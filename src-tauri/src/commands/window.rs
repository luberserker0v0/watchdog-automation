use crate::models::window::{WindowInfo, WindowState};
use crate::services;

#[derive(serde::Serialize)]
pub struct WindowDetailInfo {
    pub hwnd: usize,
    pub title: String,
    pub class_name: String,
    pub process_name: String,
    pub width: i32,
    pub height: i32,
}

#[derive(serde::Serialize)]
pub struct ResizeWindowResult {
    pub hwnd: usize,
    pub expected: [u32; 2],
    pub actual: [u32; 2],
    pub resized: bool,
}

#[tauri::command]
pub fn get_windows(
    filter: Option<WindowState>,
    desktop_id: Option<String>,
) -> Result<Vec<WindowInfo>, String> {
    services::capture::get_windows(filter, desktop_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn capture_window_by_hwnd(
    hwnd: usize,
    capture_area: Option<String>,
) -> Result<Vec<u8>, String> {
    let capture_area = match capture_area.as_deref() {
        Some("client") => crate::modules::pipeline::models::graph::CaptureArea::Client,
        Some("frame") | None => crate::modules::pipeline::models::graph::CaptureArea::Frame,
        Some(value) => return Err(format!("Invalid capture area: {}", value)),
    };
    services::capture::capture_window_by_hwnd(hwnd, capture_area).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_window_details() -> Result<Vec<WindowDetailInfo>, String> {
    let details = crate::repositories::window::get_all_window_details();
    Ok(details
        .into_iter()
        .map(|d| WindowDetailInfo {
            hwnd: d.hwnd as usize,
            title: d.title,
            class_name: d.class_name,
            process_name: d.process_name,
            width: d.width,
            height: d.height,
        })
        .collect())
}

#[tauri::command]
pub async fn resize_window_client(
    hwnd: usize,
    width: u32,
    height: u32,
    stream_state: tauri::State<'_, super::stream::StreamState>,
) -> Result<ResizeWindowResult, String> {
    if width == 0 || height == 0 {
        return Err("Window client width and height must be positive".to_string());
    }
    stream_state.0.reject_if_pipeline_owned().await?;
    let expected = [width, height];
    let resize = tokio::task::spawn_blocking(move || {
        crate::repositories::window::prepare_window_size(hwnd, expected)
    })
    .await
    .map_err(|error| format!("Window resize worker failed: {error}"))?;
    let resized = match resize {
        Ok(snapshot) => snapshot.is_some(),
        Err(mismatch) => {
            return Err(format!(
                "Could not resize '{}' to {}x{} (reached {}x{}): {}",
                mismatch.title,
                mismatch.expected[0],
                mismatch.expected[1],
                mismatch.actual[0],
                mismatch.actual[1],
                mismatch.diagnostic
            ));
        }
    };
    let actual = crate::repositories::window::get_window_client_size(hwnd)?;
    Ok(ResizeWindowResult {
        hwnd,
        expected,
        actual,
        resized,
    })
}

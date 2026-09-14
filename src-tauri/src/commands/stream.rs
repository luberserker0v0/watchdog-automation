use crate::modules::pipeline::models::graph::CaptureArea;
use crate::services::stream::{CaptureEngine, StreamService, StreamStatus, StreamTarget};
use tauri::AppHandle;

pub struct StreamState(pub std::sync::Arc<StreamService>);

#[tauri::command]
pub async fn start_stream(
    state: tauri::State<'_, StreamState>,
    app: AppHandle,
    fps: u32,
    target_type: String,
    target_index: usize,
    target_hwnd: Option<usize>,
    engine: Option<String>,
    capture_area: Option<String>,
) -> Result<(), String> {
    let target = match target_type.as_str() {
        "screen" => StreamTarget::Screen {
            index: target_index,
        },
        "window" => {
            let hwnd = target_hwnd.ok_or("hwnd is required for window target")?;
            let capture_area = match capture_area.as_deref() {
                Some("client") => CaptureArea::Client,
                Some("frame") | None => CaptureArea::Frame,
                Some(value) => return Err(format!("Invalid capture area: {}", value)),
            };
            StreamTarget::Window { hwnd, capture_area }
        }
        _ => return Err(format!("Invalid target type: {}", target_type)),
    };

    let capture_engine = match engine.as_deref() {
        Some("graphics") => CaptureEngine::Graphics,
        Some("dxgi") => CaptureEngine::Dxgi,
        _ => CaptureEngine::Gdi,
    };

    state.0.acquire_manual_owner().await?;
    if let Err(error) = state.0.start(app, fps, target, capture_engine).await {
        state.0.release_owner("manual", None).await;
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_stream(
    state: tauri::State<'_, StreamState>,
    app: AppHandle,
) -> Result<(), String> {
    state.0.reject_if_pipeline_owned().await?;
    state.0.stop_and_emit(&app, "user_stopped").await;
    state.0.release_owner("manual", None).await;
    Ok(())
}

#[tauri::command]
pub async fn get_stream_status(
    state: tauri::State<'_, StreamState>,
) -> Result<StreamStatus, String> {
    Ok(state.0.status().await)
}

use super::models::{RecorderStatus, RecordingConfig, RecordingSession};
use super::service::RecorderService;

#[tauri::command]
pub fn get_recorder_status(service: tauri::State<'_, RecorderService>) -> RecorderStatus {
    service.status()
}

#[tauri::command]
pub fn start_action_recording(
    config: RecordingConfig,
    app: tauri::AppHandle,
    service: tauri::State<'_, RecorderService>,
) -> Result<RecorderStatus, String> {
    service.start(config, app)
}

#[tauri::command]
pub fn pause_action_recording(
    app: tauri::AppHandle,
    service: tauri::State<'_, RecorderService>,
) -> Result<RecorderStatus, String> {
    service.pause(app)
}

#[tauri::command]
pub fn resume_action_recording(
    app: tauri::AppHandle,
    service: tauri::State<'_, RecorderService>,
) -> Result<RecorderStatus, String> {
    service.resume(app)
}

#[tauri::command]
pub fn stop_action_recording(
    app: tauri::AppHandle,
    service: tauri::State<'_, RecorderService>,
) -> Result<RecordingSession, String> {
    service.stop(app)
}

#[tauri::command]
pub fn clear_action_recording(service: tauri::State<'_, RecorderService>) -> Result<(), String> {
    service.clear()
}

use crate::commands::stream::StreamState;
use crate::modules::pipeline::engine::runner::run_pipeline;
use crate::modules::pipeline::engine::{PipelineStatus, RunHandle};
use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::recognizer::Recognition;
use crate::modules::pipeline::plugins::actions::click::ClickAction;
use crate::modules::pipeline::plugins::actions::input::InputTextAction;
use crate::modules::pipeline::plugins::actions::longpress::LongPressAction;
use crate::modules::pipeline::plugins::actions::press::ClickKeyAction;
use crate::modules::pipeline::plugins::actions::scroll::ScrollAction;
use crate::modules::pipeline::plugins::actions::swipe::SwipeAction;
use crate::modules::pipeline::plugins::recognizers::color_match::ColorMatchRecognizer;
use crate::modules::pipeline::plugins::recognizers::ocr::OcrRecognizer;
use crate::modules::pipeline::plugins::recognizers::template::TemplateRecognizer;
use crate::modules::pipeline::plugins::PluginRegistry;
use crate::services::stream::StreamTarget;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

lazy_static::lazy_static! {
    static ref RUN_HANDLES: Mutex<HashMap<String, Arc<RunHandle>>> = Mutex::new(HashMap::new());
}

pub fn create_registry() -> PluginRegistry {
    let mut registry = PluginRegistry::new();
    registry.register_recognizer("TemplateMatch".to_string(), Box::new(TemplateRecognizer));
    registry.register_recognizer("OCR".to_string(), Box::new(OcrRecognizer));
    registry.register_recognizer("ColorMatch".to_string(), Box::new(ColorMatchRecognizer));
    registry.register_action("Click".to_string(), Box::new(ClickAction));
    registry.register_action("InputText".to_string(), Box::new(InputTextAction));
    registry.register_action("ClickKey".to_string(), Box::new(ClickKeyAction));
    registry.register_action("Swipe".to_string(), Box::new(SwipeAction));
    registry.register_action("Scroll".to_string(), Box::new(ScrollAction));
    registry.register_action("LongPress".to_string(), Box::new(LongPressAction));
    registry
}

#[tauri::command]
pub async fn set_recognition_device(device: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        crate::modules::pipeline::plugins::recognizers::ocr::set_device(&device)
    })
    .await
    .map_err(|error| format!("OCR device worker failed: {}", error))?
}

#[tauri::command]
pub async fn start_pipeline(
    path: String,
    user_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    save_debug_images: Option<bool>,
    detailed_timings: Option<bool>,
    stream_state: tauri::State<'_, StreamState>,
    app: AppHandle,
) -> Result<String, String> {
    let base_path = std::path::Path::new(&path);
    let parent = base_path.parent().ok_or("Invalid path")?;
    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;

    let mermaid_path = parent.join(format!("{}.md", stem));
    let meta_path = parent.join(format!("{}.meta.json", stem));

    let mermaid_content = std::fs::read_to_string(&mermaid_path)
        .map_err(|e| format!("Failed to read {}: {}", mermaid_path.display(), e))?;
    let meta_content = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read {}: {}", meta_path.display(), e))?;

    let mermaid_graph = crate::modules::pipeline::parser::mermaid::parse(&mermaid_content)
        .map_err(|e| format!("Failed to parse Mermaid: {}", e))?;
    let metadata = crate::modules::pipeline::parser::meta::parse(&meta_content)
        .map_err(|e| format!("Failed to parse meta: {}", e))?;

    let graph = crate::modules::pipeline::parser::build_graph(&mermaid_graph, &metadata);

    let run_id = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let run_id_clone = run_id.clone();
    let handle = Arc::new(RunHandle::new(run_id.clone()));

    {
        let mut handles = RUN_HANDLES
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        handles.insert(run_id.clone(), handle.clone());
    }

    let registry = create_registry();
    let stream_service = stream_state.0.clone();
    let user_params = user_params.unwrap_or_default();
    let save_debug_images = save_debug_images.unwrap_or(false);
    let detailed_timings = detailed_timings.unwrap_or(false);
    let pipeline_setup_start = std::time::Instant::now();

    // Switch desktop if pipeline specifies one
    if let Some(ref desktop_id) = graph.desktop {
        let vd = crate::repositories::virtual_desktop::get_desktop_manager();
        match vd.get_current_desktop_id() {
            Ok(current_id) => {
                if current_id != *desktop_id {
                    log::info!(
                        "[runner] switching desktop: {} -> {}",
                        current_id,
                        desktop_id
                    );
                    match vd.switch_to_desktop(desktop_id) {
                        Ok(()) => {
                            if let Ok(mut orig) = handle.original_desktop.lock() {
                                *orig = Some(current_id);
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "[runner] failed to switch desktop (continuing on current): {}",
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("[runner] failed to get current desktop id: {}", e);
            }
        }
    }

    // Resolve stream target from pipeline config
    match &graph.target {
        Some(pipeline_target) => {
            // Pipeline defines a target — resolve it and ensure streaming
            let stream_target = match pipeline_target {
                crate::modules::pipeline::models::graph::PipelineTarget::Screen { index } => {
                    StreamTarget::Screen { index: *index }
                }
                crate::modules::pipeline::models::graph::PipelineTarget::Window {
                    capture_area,
                    ..
                } => {
                    let hwnd = crate::repositories::window::find_window_by_target(pipeline_target)
                        .map_err(|e| format!("Failed to resolve window target: {}", e))?;
                    StreamTarget::Window {
                        hwnd,
                        capture_area: *capture_area,
                    }
                }
            };

            // Always restart stream with pipeline's target
            if stream_service.is_streaming() {
                stream_service.stop().await;
            }
            stream_service
                .start(
                    app,
                    5,
                    stream_target,
                    crate::services::stream::CaptureEngine::Gdi,
                )
                .await;
        }
        None => {
            // No target defined — require existing stream
            if !stream_service.is_streaming() {
                return Err("Pipeline has no target and no stream is running. Start streaming first or define a target in the pipeline JSON.".to_string());
            }
        }
    }

    tokio::spawn(async move {
        let pipeline_startup_ms =
            detailed_timings.then(|| pipeline_setup_start.elapsed().as_millis() as u64);
        let _result = run_pipeline(
            graph,
            registry,
            handle.clone(),
            stream_service,
            user_params,
            save_debug_images,
            detailed_timings,
            pipeline_startup_ms,
        )
        .await;

        // Restore original desktop if we switched
        if let Ok(orig) = handle.original_desktop.lock() {
            if let Some(ref original_id) = *orig {
                log::info!("[runner] restoring desktop to {}", original_id);
                let vd = crate::repositories::virtual_desktop::get_desktop_manager();
                if let Err(e) = vd.switch_to_desktop(original_id) {
                    log::warn!("[runner] failed to restore desktop: {}", e);
                }
            }
        }

        // Keep the handle alive briefly so the frontend can poll the final status
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Ok(mut handles) = RUN_HANDLES.lock() {
            handles.remove(&run_id_clone);
        }
    });

    Ok(run_id)
}

#[tauri::command]
pub fn stop_pipeline(run_id: String) -> Result<bool, String> {
    let handles = RUN_HANDLES
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    if let Some(handle) = handles.get(&run_id) {
        handle.stop();
        Ok(true)
    } else {
        Err(format!("Pipeline run '{}' not found", run_id))
    }
}

#[tauri::command]
pub fn pause_pipeline(run_id: String) -> Result<bool, String> {
    let handles = RUN_HANDLES
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    if let Some(handle) = handles.get(&run_id) {
        let current = handle.get_state();
        use crate::modules::pipeline::engine::PipelineState;
        let new_state = match current {
            PipelineState::Running => PipelineState::Paused,
            PipelineState::Paused => PipelineState::Running,
            _ => return Err(format!("Cannot pause pipeline in state: {:?}", current)),
        };
        {
            if let Ok(mut state) = handle.state.lock() {
                *state = new_state;
            }
        }
        Ok(true)
    } else {
        Err(format!("Pipeline run '{}' not found", run_id))
    }
}

#[tauri::command]
pub fn get_pipeline_status(run_id: String) -> Result<PipelineStatus, String> {
    let handles = RUN_HANDLES
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    if let Some(handle) = handles.get(&run_id) {
        Ok(handle.status())
    } else {
        Err(format!("Pipeline run '{}' not found", run_id))
    }
}

#[tauri::command]
pub fn list_running_pipelines() -> Result<Vec<PipelineStatus>, String> {
    let handles = RUN_HANDLES
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    let statuses: Vec<PipelineStatus> = handles.values().map(|handle| handle.status()).collect();
    Ok(statuses)
}

#[tauri::command]
pub async fn test_action(action: Action) -> Result<String, String> {
    log::info!("[node_test] testing action {:?}", action);
    let registry = create_registry();
    let result = registry
        .execute_action(&action, None, &HashMap::new())
        .await;
    match &result {
        Ok(()) => log::info!("[node_test] action executed successfully"),
        Err(e) => log::warn!("[node_test] action error: {}", e),
    }
    result
        .map(|_| "Action executed successfully".to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_recognition(
    recognition: Recognition,
    roi: Option<[i32; 4]>,
    candidate_selection: Option<crate::modules::pipeline::models::recognizer::CandidateSelection>,
    image_bytes: Option<Vec<u8>>,
) -> Result<serde_json::Value, String> {
    log::info!(
        "[node_test] testing recognition {:?}, roi={:?}",
        recognition,
        roi
    );
    tokio::task::spawn_blocking(move || {
        let bytes = match image_bytes {
            Some(b) if !b.is_empty() => b,
            _ => {
                let cache = crate::FRAME_CACHE
                    .lock()
                    .map_err(|e| format!("Frame cache lock error: {}", e))?;
                cache
                    .as_ref()
                    .map(|frame| frame.bytes.clone())
                    .ok_or_else(|| {
                        "No frame available. Start streaming or capture a screenshot first."
                            .to_string()
                    })?
            }
        };

        let img = image::load_from_memory(&bytes)
            .map_err(|e| format!("Failed to decode image: {}", e))?;

        let registry = create_registry();
        let result = registry
            .evaluate_recognition_with_selection(
                &img,
                &recognition,
                roi.as_ref(),
                candidate_selection.as_ref(),
            )
            .map_err(|e| e.to_string())?;

        log::info!(
            "[node_test] recognition result: matched={}, confidence={:.3}",
            result.matched,
            result.confidence
        );
        serde_json::to_value(&result).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

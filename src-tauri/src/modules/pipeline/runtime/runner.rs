use crate::modules::capture::commands::stream::StreamState;
use crate::modules::capture::services::stream::StreamTarget;
use crate::modules::pipeline::authoring::models::action::Action;
use crate::modules::pipeline::authoring::models::graph::{
    CaptureArea, LaunchConfig, PipelineGraph, PipelineTarget,
};
use crate::modules::pipeline::authoring::models::recognizer::Recognition;
use crate::modules::pipeline::authoring::parser::meta::MetadataFile;
use crate::modules::pipeline::plugins::actions::click::ClickAction;
use crate::modules::pipeline::plugins::actions::input::InputTextAction;
use crate::modules::pipeline::plugins::actions::longpress::LongPressAction;
use crate::modules::pipeline::plugins::actions::press::ClickKeyAction;
use crate::modules::pipeline::plugins::actions::scroll::ScrollAction;
use crate::modules::pipeline::plugins::actions::swipe::SwipeAction;
use crate::modules::pipeline::plugins::recognizers::color_match::ColorMatchRecognizer;
use crate::modules::pipeline::plugins::recognizers::ocr::OcrRecognizer;
use crate::modules::pipeline::plugins::recognizers::template::TemplateRecognizer;
use crate::modules::pipeline::plugins::shared::{self, CaptureBounds};
use crate::modules::pipeline::plugins::PluginRegistry;
use crate::modules::pipeline::runtime::engine::runner::run_pipeline;
use crate::modules::pipeline::runtime::engine::{PipelineStatus, RunHandle};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

lazy_static::lazy_static! {
    static ref RUN_HANDLES: Mutex<HashMap<String, Arc<RunHandle>>> = Mutex::new(HashMap::new());
}
static PIPELINE_START_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum PipelineStartError {
    Generic {
        message: String,
    },
    WindowSizeMismatch {
        hwnd: usize,
        title: String,
        expected: [u32; 2],
        before: [u32; 2],
        actual: [u32; 2],
        resize_attempted: bool,
        diagnostic: String,
    },
}

impl PipelineStartError {
    fn diagnostic(&self) -> &str {
        match self {
            Self::Generic { message } => message,
            Self::WindowSizeMismatch { diagnostic, .. } => diagnostic,
        }
    }
}

impl From<String> for PipelineStartError {
    fn from(message: String) -> Self {
        Self::Generic { message }
    }
}

impl From<&str> for PipelineStartError {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

impl From<crate::modules::capture::repositories::window::WindowSizeMismatch>
    for PipelineStartError
{
    fn from(mismatch: crate::modules::capture::repositories::window::WindowSizeMismatch) -> Self {
        Self::WindowSizeMismatch {
            hwnd: mismatch.hwnd,
            title: mismatch.title,
            expected: mismatch.expected,
            before: mismatch.before,
            actual: mismatch.actual,
            resize_attempted: mismatch.resize_attempted,
            diagnostic: mismatch.diagnostic,
        }
    }
}

fn restore_on_start_error(
    mut error: PipelineStartError,
    snapshot: Option<&crate::modules::capture::repositories::window::WindowPlacementSnapshot>,
) -> PipelineStartError {
    if let Some(snapshot) = snapshot {
        if let Err(cleanup_error) =
            crate::modules::capture::repositories::window::restore_window_placement(snapshot)
        {
            log::error!("[runner] startup cleanup failed: {cleanup_error}");
            match &mut error {
                PipelineStartError::Generic { message } => {
                    message.push_str(&format!("; cleanup failed: {cleanup_error}"))
                }
                PipelineStartError::WindowSizeMismatch { diagnostic, .. } => {
                    diagnostic.push_str(&format!("; cleanup failed: {cleanup_error}"))
                }
            }
        }
    }
    error
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

fn resolve_pipeline_params(
    graph: &PipelineGraph,
    metadata: &MetadataFile,
    supplied: HashMap<String, HashMap<String, serde_json::Value>>,
) -> HashMap<String, HashMap<String, serde_json::Value>> {
    let mut resolved = HashMap::new();
    for node in &graph.nodes {
        let mut values: HashMap<String, serde_json::Value> = node
            .params
            .iter()
            .filter(|param| !param.default.is_null())
            .map(|param| (param.key.clone(), param.default.clone()))
            .collect();
        if let Some(instance) = metadata
            .instances
            .as_ref()
            .and_then(|instances| instances.get(&node.id))
        {
            values.extend(instance.param_values.clone());
        }
        if let Some(explicit) = supplied.get(&node.id) {
            values.extend(explicit.clone());
        }
        if !values.is_empty() {
            resolved.insert(node.id.clone(), values);
        }
    }
    resolved
}

/// Starts the declared process once startup has established that a matching
/// target window does not already exist. Keeping this before capture ownership
/// makes `launch + target` an atomic startup concern rather than an execution
/// side effect.
fn launch_process(config: &LaunchConfig) -> Result<(), String> {
    let mut command = match (&config.exe, &config.cmd) {
        (Some(_), Some(_)) => {
            return Err("launch must specify exactly one of exe or cmd, not both".to_string())
        }
        (None, None) => return Err("launch must specify one of exe or cmd".to_string()),
        (Some(exe), None) => std::process::Command::new(exe),
        (None, Some(cmd)) if config.shell => {
            let mut command = std::process::Command::new("cmd");
            command.args(["/C", cmd]);
            command
        }
        (None, Some(cmd)) => std::process::Command::new(cmd),
    };
    command.args(&config.args);
    if let Some(cwd) = &config.cwd {
        command.current_dir(cwd);
    }
    command.envs(&config.env);
    command
        .spawn()
        .map(|child| {
            log::info!("[pipeline] launched external process pid={}", child.id());
        })
        .map_err(|error| format!("Failed to launch configured process: {error}"))
}

async fn resolve_or_launch_window_target(
    target: &PipelineTarget,
    launch: Option<&LaunchConfig>,
) -> Result<usize, String> {
    if !target.has_window_filters() {
        return Err("Filterless window target requires an active window stream; it cannot safely be paired with launch because no window identity is available to verify".to_string());
    }

    match crate::modules::capture::repositories::window::find_window_by_target(target) {
        Ok(hwnd) => {
            log::info!("[pipeline] reusing existing target window hwnd={hwnd}");
            return Ok(hwnd);
        }
        Err(error) if launch.is_none() => {
            return Err(format!("Failed to resolve window target: {error}"))
        }
        Err(error) => {
            log::info!("[pipeline] target window not found ({error}); launching configured process")
        }
    }

    let config = launch.expect("launch checked above").clone();
    let launch_config = config.clone();
    tokio::task::spawn_blocking(move || launch_process(&launch_config))
        .await
        .map_err(|error| format!("Launch worker failed: {error}"))??;

    // An immediate lookup preserves the explicit `wait_for_window: false`
    // contract. Waiting is opt-in because some applications intentionally
    // launch without creating a top-level window.
    let timeout_ms = config
        .wait_for_window
        .then(|| config.wait_timeout_ms.unwrap_or(10_000))
        .unwrap_or(0);
    let started = std::time::Instant::now();
    loop {
        if let Ok(hwnd) =
            crate::modules::capture::repositories::window::find_window_by_target(target)
        {
            log::info!("[pipeline] launched target window resolved hwnd={hwnd}");
            return Ok(hwnd);
        }
        if started.elapsed().as_millis() as u64 >= timeout_ms {
            return Err(if timeout_ms == 0 {
                "Configured process launched, but its target window is not ready. Set launch.wait_for_window to true and configure a window target filter.".to_string()
            } else {
                format!("Target window did not appear within {timeout_ms}ms after launch")
            });
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
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
    user_params: Option<
        std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
    >,
    save_debug_images: Option<bool>,
    detailed_timings: Option<bool>,
    patrol_config: Option<crate::modules::pipeline::runtime::patrol::PatrolConfig>,
    vlm_config: Option<crate::modules::pipeline::runtime::patrol::VlmConnection>,
    stream_state: tauri::State<'_, StreamState>,
    app: AppHandle,
) -> Result<String, PipelineStartError> {
    let result = start_pipeline_inner(
        path.clone(),
        user_params,
        save_debug_images,
        detailed_timings,
        patrol_config,
        vlm_config,
        stream_state,
        app,
    )
    .await;
    if let Err(error) = &result {
        log::error!(
            "[pipeline] STARTUP FAILED: path='{}', reason='{}'",
            path,
            error.diagnostic()
        );
    }
    result
}

async fn start_pipeline_inner(
    path: String,
    user_params: Option<
        std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
    >,
    save_debug_images: Option<bool>,
    detailed_timings: Option<bool>,
    patrol_config: Option<crate::modules::pipeline::runtime::patrol::PatrolConfig>,
    vlm_config: Option<crate::modules::pipeline::runtime::patrol::VlmConnection>,
    stream_state: tauri::State<'_, StreamState>,
    app: AppHandle,
) -> Result<String, PipelineStartError> {
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

    let mermaid_graph =
        crate::modules::pipeline::authoring::parser::mermaid::parse(&mermaid_content)
            .map_err(|e| format!("Failed to parse Mermaid: {}", e))?;
    let metadata = crate::modules::pipeline::authoring::parser::meta::parse(&meta_content)
        .map_err(|e| format!("Failed to parse {}: {}", meta_path.display(), e))?;

    let layout_path = parent.join(format!("{stem}.layout.json"));
    if layout_path.is_file() {
        let content = std::fs::read_to_string(&layout_path)
            .map_err(|error| format!("Failed to read {}: {error}", layout_path.display()))?;
        let layout = crate::modules::pipeline::authoring::parser::layout::parse(&content)
            .map_err(|error| format!("Invalid {}: {error}", layout_path.display()))?;
        crate::modules::pipeline::authoring::editor::validate_layout_nodes(&layout, &mermaid_graph)
            .map_err(|error| format!("Invalid {}: {error}", layout_path.display()))?;
    }

    let graph = crate::modules::pipeline::authoring::parser::build_graph(&mermaid_graph, &metadata);
    let mut validation = crate::modules::pipeline::authoring::editor::validate_pipeline_sources(
        &mermaid_graph,
        &metadata,
    );
    validation.extend(crate::modules::pipeline::authoring::editor::validate_pipeline_graph(&graph));
    if !validation.is_empty() {
        return Err(format!(
            "Invalid pipeline: {}",
            crate::modules::pipeline::authoring::editor::format_validation_errors(&validation)
        )
        .into());
    }
    // Compile routes once before touching desktop, window, or capture state.
    // The runner keeps the graph-shaped API for compatibility, while the
    // immutable plan supplies ordered children/error routes to the engine.
    let execution_plan = crate::modules::pipeline::runtime::plan::ExecutionPlan::compile(
        graph.clone(),
    )
    .map_err(|errors| PipelineStartError::Generic {
        message: format!("Invalid execution plan: {}", errors.join("; ")),
    })?;
    let run_id = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let patrol_config = patrol_config.unwrap_or_default();
    let patrol = if patrol_config.enabled {
        match crate::modules::pipeline::runtime::patrol::PatrolRuntime::new(
            patrol_config,
            &mermaid_path,
            &run_id,
            vlm_config,
        ) {
            Ok(runtime) => Some(runtime),
            Err(error) => {
                log::warn!("[patrol] unavailable for this run: {error}");
                None
            }
        }
    } else {
        None
    };
    let _start_guard = PIPELINE_START_LOCK.lock().await;
    // Do not rely on `tauri::State` field autoderef here: the background
    // runner owns an Arc<StreamService>, not a borrowed StreamState.
    let stream_service: Arc<crate::modules::capture::services::stream::StreamService> =
        Arc::clone(&stream_state.inner().0);
    let user_params = resolve_pipeline_params(&graph, &metadata, user_params.unwrap_or_default());
    let save_debug_images = save_debug_images.unwrap_or(false);
    let detailed_timings = detailed_timings.unwrap_or(false);
    let pipeline_setup_start = std::time::Instant::now();

    // Window-size preparation happens before a run is registered or capture
    // ownership is transferred.

    let initial_target = graph
        .target
        .as_ref()
        .or_else(|| graph.entry_node().and_then(|node| node.target.as_ref()));

    // A filtered window target is resolved by `resolve_or_launch_window_target`
    // below so an existing app is reused. Screen/no-target pipelines have no
    // window identity to probe, therefore their explicit launch remains an
    // unconditional setup action. A filterless window cannot be verified after
    // launch and must continue to use an already active manual stream.
    if let Some(launch) = graph.launch.as_ref() {
        match initial_target {
            Some(target @ PipelineTarget::Window { .. }) if !target.has_window_filters() => {
                return Err("launch requires a window target with process, class, or title filters; a filterless window target can only reuse an active window stream".into());
            }
            Some(PipelineTarget::Window { .. }) => {}
            _ => {
                let launch = launch.clone();
                if let Err(error) = tokio::task::spawn_blocking(move || launch_process(&launch))
                    .await
                    .map_err(|error| format!("Launch worker failed: {error}"))
                    .and_then(|result| result)
                {
                    return Err(error.into());
                }
            }
        }
    }
    let resolved_stream_target = match initial_target {
        Some(pipeline_target) => {
            let target = match pipeline_target {
                PipelineTarget::Screen { index } => StreamTarget::Screen { index: *index },
                PipelineTarget::Window { capture_area, .. } => {
                    let hwnd = if pipeline_target.has_window_filters() {
                        match resolve_or_launch_window_target(
                            pipeline_target,
                            graph.launch.as_ref(),
                        )
                        .await
                        {
                            Ok(hwnd) => hwnd,
                            Err(error) => {
                                return Err(error.into());
                            }
                        }
                    } else {
                        let status = stream_service.status().await;
                        if status.is_streaming && status.target_type == "window" {
                            status.target_index
                        } else {
                            return Err(
                                "Filterless window target requires an active window stream".into(),
                            );
                        }
                    };
                    StreamTarget::Window {
                        hwnd,
                        capture_area: *capture_area,
                    }
                }
            };
            Some(target)
        }
        None => {
            if !stream_service.is_streaming() {
                return Err("Pipeline has no target and no stream is running. Start streaming first or define a target in the pipeline JSON.".into());
            }
            None
        }
    };

    let mut window_restore = None;
    if let (
        Some(crate::modules::pipeline::authoring::models::graph::PipelineTarget::Window {
            window_size: Some(expected),
            ..
        }),
        Some(StreamTarget::Window { hwnd, .. }),
    ) = (graph.target.as_ref(), resolved_stream_target.as_ref())
    {
        let hwnd = *hwnd;
        let expected = *expected;
        let preflight = match tokio::task::spawn_blocking(move || {
            crate::modules::capture::repositories::window::prepare_window_size(hwnd, expected)
        })
        .await
        {
            Ok(result) => result,
            Err(error) => {
                return Err(PipelineStartError::Generic {
                    message: format!("Window-size preflight worker failed: {error}"),
                });
            }
        };
        match preflight {
            Ok(snapshot) => window_restore = snapshot,
            Err(mismatch) => {
                return Err(mismatch.into());
            }
        }
    }

    if let Some(StreamTarget::Window { hwnd, .. }) = resolved_stream_target.as_ref() {
        let hwnd = *hwnd;
        let foreground_result = tokio::task::spawn_blocking(move || {
            crate::modules::capture::repositories::window::ensure_window_foreground(hwnd)
        })
        .await;
        let foreground_error = match foreground_result {
            Ok(Ok(_)) => None,
            Ok(Err(error)) => Some(error),
            Err(error) => Some(format!("Target foreground worker failed: {error}")),
        };
        if let Some(error) = foreground_error {
            return Err(restore_on_start_error(
                format!("target_not_foreground: {error}").into(),
                window_restore.as_ref(),
            ));
        }
    }

    let run_id_clone = run_id.clone();
    let handle = Arc::new(RunHandle::new(run_id.clone()));
    let registry = create_registry();

    if let Err(error) = stream_service.acquire_pipeline_owner(&run_id).await {
        return Err(restore_on_start_error(
            error.into(),
            window_restore.as_ref(),
        ));
    }

    if let Some(stream_target) = resolved_stream_target {
        let frame_size_verification = match (graph.target.as_ref(), &stream_target) {
            (
                Some(crate::modules::pipeline::authoring::models::graph::PipelineTarget::Window {
                    capture_area: CaptureArea::Client,
                    window_size: Some(expected),
                    title,
                    ..
                }),
                StreamTarget::Window { .. },
            ) => Some((*expected, title.clone().unwrap_or_default())),
            _ => None,
        };
        let mut verification_receiver = frame_size_verification
            .as_ref()
            .map(|_| stream_service.subscribe());
        if stream_service.is_streaming() {
            stream_service.stop().await;
        }
        if let Err(error) = stream_service
            .start(
                app.clone(),
                5,
                stream_target.clone(),
                crate::modules::capture::services::stream::CaptureEngine::Gdi,
            )
            .await
        {
            stream_service.release_pipeline_owner(&run_id).await;
            return Err(restore_on_start_error(
                error.into(),
                window_restore.as_ref(),
            ));
        }
        if let (Some((expected, title)), Some(receiver)) =
            (frame_size_verification, verification_receiver.as_mut())
        {
            let session_id = stream_service.status().await.session_id;
            let frame = tokio::time::timeout(std::time::Duration::from_secs(3), async {
                loop {
                    match receiver.recv().await {
                        Ok(frame)
                            if frame.session_id == session_id && frame.target == stream_target =>
                        {
                            break Ok(frame)
                        }
                        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            continue
                        }
                        Err(error) => break Err(format!("Stream receive failed: {error}")),
                    }
                }
            })
            .await;
            let verification_error = match frame {
                Ok(Ok(frame))
                    if !crate::modules::capture::repositories::window::window_size_matches(
                        [frame.width, frame.height],
                        expected,
                    ) => Some(PipelineStartError::WindowSizeMismatch {
                        hwnd: match &stream_target {
                            StreamTarget::Window { hwnd, .. } => *hwnd,
                            StreamTarget::Screen { .. } => 0,
                        },
                        title,
                        expected,
                        before: window_restore
                            .as_ref()
                            .map(|snapshot| snapshot.original_client_size)
                            .unwrap_or(expected),
                        actual: [frame.width, frame.height],
                        resize_attempted: true,
                        diagnostic: "The encoded client frame does not match the verified window client size".to_string(),
                    }),
                Ok(Err(error)) => Some(PipelineStartError::Generic { message: error }),
                Err(_) => Some(PipelineStartError::Generic {
                    message: "Timed out while verifying the first client-area frame".to_string(),
                }),
                _ => None,
            };
            if let Some(error) = verification_error {
                stream_service
                    .stop_and_emit(&app, "pipeline_start_failed")
                    .await;
                stream_service.release_pipeline_owner(&run_id).await;
                return Err(restore_on_start_error(error, window_restore.as_ref()));
            }
        }
        let (target_type, hwnd) = match stream_target {
            StreamTarget::Screen { .. } => ("screen", None),
            StreamTarget::Window { hwnd, .. } => ("window", Some(hwnd)),
        };
        let _ = app.emit(
            "update_capture",
            serde_json::json!({
                "type": target_type,
                "action": "sync",
                "target": initial_target,
                "hwnd": hwnd,
            }),
        );
    }

    let insertion_error = match RUN_HANDLES.lock() {
        Ok(mut handles) => {
            handles.insert(run_id.clone(), handle.clone());
            None
        }
        Err(error) => Some(format!("Lock error: {error}")),
    };
    if let Some(error) = insertion_error {
        stream_service
            .stop_and_emit(&app, "pipeline_start_failed")
            .await;
        stream_service.release_pipeline_owner(&run_id).await;
        return Err(restore_on_start_error(
            error.into(),
            window_restore.as_ref(),
        ));
    }

    tokio::spawn(async move {
        let pipeline_startup_ms =
            detailed_timings.then(|| pipeline_setup_start.elapsed().as_millis() as u64);
        let execution_handle = handle.clone();
        let execution_stream = stream_service.clone();
        let execution_app = app.clone();
        let result = match tokio::spawn(run_pipeline(
            execution_plan,
            registry,
            execution_handle,
            execution_stream,
            user_params,
            save_debug_images,
            detailed_timings,
            pipeline_startup_ms,
            patrol,
            execution_app,
        ))
        .await
        {
            Ok(result) => result,
            Err(error) => {
                let message = format!("Pipeline worker terminated unexpectedly: {error}");
                log::error!("[pipeline] FAILED: {message}");
                if let Ok(mut state) = handle.state.lock() {
                    *state = crate::modules::pipeline::runtime::engine::PipelineState::Failed(
                        message.clone(),
                    );
                }
                crate::modules::pipeline::runtime::engine::EngineResult {
                    success: false,
                    steps: Vec::new(),
                    error: Some(message),
                }
            }
        };

        stream_service
            .stop_and_emit(&app, "pipeline_finished")
            .await;
        if result.success {
            log::info!(
                "[runner] completed run {}; pipeline-owned stream released",
                run_id_clone
            );
        } else {
            log::error!(
                "[runner] FAILED run {}; pipeline-owned stream released: {}",
                run_id_clone,
                result.error.as_deref().unwrap_or("no failure diagnostic")
            );
        }
        stream_service.release_pipeline_owner(&run_id_clone).await;

        if let Some(snapshot) = &window_restore {
            if let Err(error) =
                crate::modules::capture::repositories::window::restore_window_placement(snapshot)
            {
                log::error!("[runner] run cleanup failed: {error}");
                if let Ok(mut cleanup_error) = handle.cleanup_error.lock() {
                    *cleanup_error = Some(error);
                }
            }
        }

        handle
            .cleanup_complete
            .store(true, std::sync::atomic::Ordering::Release);
        let _ = app.emit("pipeline-status", handle.event(None));

        // Keep the handle alive briefly so the frontend can poll the final status
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        if let Ok(mut handles) = RUN_HANDLES.lock() {
            handles.remove(&run_id_clone);
        }
    });

    Ok(run_id)
}

#[tauri::command]
pub fn stop_pipeline(run_id: String, app: AppHandle) -> Result<bool, String> {
    let handles = RUN_HANDLES
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    if let Some(handle) = handles.get(&run_id) {
        handle.stop();
        let _ = app.emit("pipeline-status", handle.event(None));
        Ok(true)
    } else {
        Err(format!("Pipeline run '{}' not found", run_id))
    }
}

#[tauri::command]
pub fn pause_pipeline(run_id: String, app: AppHandle) -> Result<bool, String> {
    let handles = RUN_HANDLES
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    if let Some(handle) = handles.get(&run_id) {
        let current = handle.get_state();
        use crate::modules::pipeline::runtime::engine::PipelineState;
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
        let _ = app.emit("pipeline-status", handle.event(None));
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ActionTestContext {
    Screen {
        index: usize,
        frame_width: u32,
        frame_height: u32,
    },
    Window {
        hwnd: usize,
        #[serde(default)]
        capture_area: CaptureArea,
        frame_width: u32,
        frame_height: u32,
    },
}

fn action_context_matches_stream(
    context: &ActionTestContext,
    status: &crate::modules::capture::services::stream::StreamStatus,
) -> bool {
    match context {
        ActionTestContext::Screen {
            index,
            frame_width,
            frame_height,
        } => {
            status.is_streaming
                && status.target_type == "screen"
                && status.target_index == *index
                && status.frame_width == Some(*frame_width)
                && status.frame_height == Some(*frame_height)
        }
        ActionTestContext::Window {
            hwnd,
            capture_area,
            frame_width,
            frame_height,
        } => {
            status.is_streaming
                && status.target_type == "window"
                && status.target_index == *hwnd
                && status.capture_area
                    == match capture_area {
                        CaptureArea::Frame => "frame",
                        CaptureArea::Client => "client",
                    }
                && status.frame_width == Some(*frame_width)
                && status.frame_height == Some(*frame_height)
        }
    }
}

fn map_action_with_bounds(
    action: Action,
    frame_width: u32,
    frame_height: u32,
    bounds: CaptureBounds,
) -> Result<Action, String> {
    if frame_width == 0 || frame_height == 0 {
        return Err("Action test frame dimensions must be greater than zero".to_string());
    }

    shared::map_action_checked(
        &action,
        None,
        &HashMap::new(),
        frame_width,
        frame_height,
        bounds,
    )
    .map_err(|error| error.to_string())
}

fn map_test_action(action: Action, context: ActionTestContext) -> Result<Action, String> {
    let (frame_width, frame_height, bounds) = match context {
        ActionTestContext::Screen {
            index,
            frame_width,
            frame_height,
        } => (
            frame_width,
            frame_height,
            shared::screen_capture_bounds(index)?,
        ),
        ActionTestContext::Window {
            hwnd,
            capture_area,
            frame_width,
            frame_height,
        } => {
            crate::modules::capture::repositories::window::ensure_window_foreground(hwnd)
                .map_err(|error| format!("target_not_foreground: {error}"))?;
            (
                frame_width,
                frame_height,
                shared::window_capture_bounds(hwnd, capture_area, frame_width, frame_height)?,
            )
        }
    };
    map_action_with_bounds(action, frame_width, frame_height, bounds)
}

#[tauri::command]
pub async fn test_action(
    action: Action,
    context: Option<ActionTestContext>,
    stream_state: tauri::State<'_, StreamState>,
) -> Result<String, String> {
    stream_state.inner().0.reject_if_pipeline_owned().await?;
    let context = context.ok_or("Action test requires an active screen or window context")?;
    let status = stream_state.inner().0.status().await;
    if !action_context_matches_stream(&context, &status) {
        return Err("Action test context does not match the active capture stream".into());
    }
    let window_hwnd = match &context {
        ActionTestContext::Window { hwnd, .. } => Some(*hwnd),
        ActionTestContext::Screen { .. } => None,
    };
    let action = map_test_action(action, context)?;
    log::info!("[node_test] testing mapped action {:?}", action);
    if let Some(hwnd) = window_hwnd {
        crate::modules::capture::repositories::window::require_window_foreground(hwnd)
            .map_err(|error| format!("target_not_foreground: {error}"))?;
    }
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

#[cfg(test)]
mod action_test_tests {
    use super::*;

    #[test]
    fn action_context_requires_the_active_capture_target_and_frame() {
        let mut status = crate::modules::capture::services::stream::StreamStatus {
            is_streaming: true,
            fps: 30,
            target_type: "window".into(),
            target_index: 42,
            start_time: None,
            engine: "test".into(),
            capture_area: "client".into(),
            owner: "manual".into(),
            owner_id: None,
            session_id: 1,
            frame_width: Some(800),
            frame_height: Some(600),
            client_width: Some(800),
            client_height: Some(600),
        };
        let context = ActionTestContext::Window {
            hwnd: 42,
            capture_area: CaptureArea::Client,
            frame_width: 800,
            frame_height: 600,
        };
        assert!(action_context_matches_stream(&context, &status));
        status.target_index = 43;
        assert!(!action_context_matches_stream(&context, &status));
        status.target_index = 42;
        status.frame_width = Some(801);
        assert!(!action_context_matches_stream(&context, &status));
        status.frame_width = Some(800);
        status.is_streaming = false;
        assert!(!action_context_matches_stream(&context, &status));
    }

    #[test]
    fn pipeline_parameters_use_definition_then_instance_then_explicit_values() {
        let topology =
            crate::modules::pipeline::authoring::parser::mermaid::parse("flowchart TD\nA[\"A\"]")
                .unwrap();
        let metadata = crate::modules::pipeline::authoring::parser::meta::parse(r#"{"nodes":{"T":{"kind":"Entry"}},"instances":{"A":{"node":"T","params":[{"type":"text","key":"name","label":"Name","default":"Default"}],"param_values":{"name":"Alice","hidden":"value"}}}}"#).unwrap();
        let graph = crate::modules::pipeline::authoring::parser::build_graph(&topology, &metadata);
        let empty = resolve_pipeline_params(&graph, &metadata, HashMap::new());
        assert_eq!(empty["A"]["name"], serde_json::json!("Alice"));
        assert_eq!(empty["A"]["hidden"], serde_json::json!("value"));
        let explicit = HashMap::from([(
            "A".into(),
            HashMap::from([("name".into(), serde_json::json!("Bob"))]),
        )]);
        let resolved = resolve_pipeline_params(&graph, &metadata, explicit);
        assert_eq!(resolved["A"]["name"], serde_json::json!("Bob"));
    }
    use crate::modules::pipeline::authoring::models::action::Target;

    fn point(action: Action) -> [i32; 2] {
        match action {
            Action::Click {
                target: Target::Coordinate(point),
                target_offset: None,
            } => point,
            other => panic!("expected mapped click point, got {:?}", other),
        }
    }

    #[test]
    fn action_test_maps_capture_coordinates_and_offsets_to_screen() {
        let action = Action::Click {
            target: Target::Coordinate([50, 25]),
            target_offset: Some([10, 5, 0, 0]),
        };
        let mapped = map_action_with_bounds(
            action,
            100,
            50,
            CaptureBounds {
                left: -200,
                top: 100,
                width: 200,
                height: 100,
            },
        )
        .unwrap();

        assert_eq!(point(mapped), [-80, 160]);
    }

    #[test]
    fn frame_and_client_bounds_produce_distinct_action_positions() {
        let action = Action::Click {
            target: Target::Coordinate([10, 20]),
            target_offset: None,
        };
        let frame = map_action_with_bounds(
            action.clone(),
            100,
            100,
            CaptureBounds {
                left: 40,
                top: 50,
                width: 100,
                height: 100,
            },
        )
        .unwrap();
        let client = map_action_with_bounds(
            action,
            100,
            100,
            CaptureBounds {
                left: 48,
                top: 82,
                width: 100,
                height: 100,
            },
        )
        .unwrap();

        assert_eq!(point(frame), [50, 70]);
        assert_eq!(point(client), [58, 102]);
    }

    #[test]
    fn action_test_rejects_targets_outside_the_encoded_frame() {
        let action = Action::Swipe {
            begin: Target::Coordinate([10, 20]),
            begin_offset: None,
            end: Target::Coordinate([100, 20]),
            end_offset: None,
            duration: 200,
        };
        let error = map_action_with_bounds(
            action,
            100,
            100,
            CaptureBounds {
                left: 40,
                top: 50,
                width: 100,
                height: 100,
            },
        )
        .unwrap_err();
        assert!(error.contains("outside capture frame"));
    }

    #[test]
    fn window_size_mismatch_is_a_tagged_start_error() {
        let value = serde_json::to_value(PipelineStartError::WindowSizeMismatch {
            hwnd: 42,
            title: "Target".to_string(),
            expected: [1280, 720],
            before: [900, 600],
            actual: [900, 600],
            resize_attempted: true,
            diagnostic: "rejected".to_string(),
        })
        .unwrap();
        assert_eq!(value["code"], "window_size_mismatch");
        assert_eq!(value["expected"], serde_json::json!([1280, 720]));
    }
}

#[tauri::command]
pub async fn test_recognition(
    recognition: Recognition,
    roi: Option<[i32; 4]>,
    candidate_selection: Option<
        crate::modules::pipeline::authoring::models::recognizer::CandidateSelection,
    >,
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
        let mut result = registry
            .evaluate_recognition_with_selection(
                &img,
                &recognition,
                roi.as_ref(),
                candidate_selection.as_ref(),
            )
            .map_err(|e| e.to_string())?;
        crate::modules::pipeline::authoring::models::recognizer::limit_debug_candidates(
            &mut result.candidates,
            100,
        );

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

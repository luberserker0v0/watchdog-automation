use crate::modules::pipeline::commands::runner::create_registry;
use crate::modules::pipeline::engine::debug_image;
use crate::modules::pipeline::models::graph::PipelineTarget;
use crate::modules::pipeline::models::node::NodeConfig;
use crate::services::stream::{CaptureEngine, StreamStatus, StreamTarget};
use crate::utils::paths;
use tauri::Emitter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMetaInfo {
    pub id: String,
    pub file_name: String,
    pub source: String,
    #[serde(flatten)]
    pub config: NodeConfig,
}

impl std::ops::Deref for NodeMetaInfo {
    type Target = NodeConfig;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeTestResult {
    pub success: bool,
    pub matched: bool,
    pub confidence: f64,
    pub box_region: Option<crate::modules::pipeline::models::recognizer::Region>,
    pub actions_fired: u32,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub candidates: Vec<crate::modules::pipeline::models::recognizer::MatchCandidate>,
    pub selection_error: Option<String>,
    pub screen_click_position: Option<[i32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings_ms: Option<std::collections::BTreeMap<String, u64>>,
}

#[derive(Clone, serde::Serialize)]
struct UpdateCapturePayload {
    r#type: String,
    action: String,
    target: PipelineTarget,
    hwnd: Option<usize>,
}

const NODE_META_SUFFIX: &str = ".meta.json";

fn node_meta_file_name(path: &std::path::Path) -> Option<String> {
    path.file_name()?
        .to_str()?
        .strip_suffix(NODE_META_SUFFIX)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

fn read_node_metadata(
    path: &std::path::Path,
) -> Result<crate::modules::pipeline::parser::meta::MetadataFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    crate::modules::pipeline::parser::meta::parse(&content)
        .map_err(|e| format!("Failed to deserialize {}: {}", path.display(), e))
}

#[tauri::command]
pub fn list_node_metas() -> Result<Vec<NodeMetaInfo>, String> {
    let built_in_dir = paths::built_in_pipeline_nodes_dir();
    let custom_dir = paths::user_pipeline_nodes_dir();
    log::info!(
        "[node_test] listing node metas from {} and {}",
        built_in_dir.display(),
        custom_dir.display()
    );
    let mut nodes = Vec::new();
    append_node_metas(&built_in_dir, "built-in", &mut nodes)?;
    append_node_metas(&custom_dir, "custom", &mut nodes)?;
    nodes.sort_by(|a, b| (&a.source, &a.id).cmp(&(&b.source, &b.id)));
    log::info!("[node_test] found {} node metas", nodes.len());
    Ok(nodes)
}

fn append_node_metas(
    dir: &std::path::Path,
    source: &str,
    nodes: &mut Vec<NodeMetaInfo>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(&dir).map_err(|e| {
        let msg = format!("Failed to read node metas dir: {}", e);
        log::warn!("[node_test] {}", msg);
        msg
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = node_meta_file_name(&path) else {
            continue;
        };

        let metadata = read_node_metadata(&path)?;
        nodes.extend(metadata.nodes.into_iter().map(|(id, config)| NodeMetaInfo {
            id,
            file_name: file_name.clone(),
            source: source.to_string(),
            config,
        }));
    }
    Ok(())
}

fn bring_to_front(hwnd: usize) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        let hwnd_val = windows::Win32::Foundation::HWND(hwnd as *mut _);
        let _ = ShowWindow(hwnd_val, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd_val);
    }
}

fn launch_target_app(target: &PipelineTarget) -> Result<std::process::Child, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let dir = exe.parent().ok_or("Failed to get exe parent dir")?;

    // Map target window title to the window label for watch-dog-target.exe
    let label = match target {
        PipelineTarget::Window {
            title: Some(title), ..
        } => {
            if title.contains("Login") {
                "login"
            } else if title.contains("Dashboard") {
                "dashboard"
            } else if title.contains("Settings") {
                "settings"
            } else {
                ""
            }
        }
        _ => "",
    };

    let file_name = if cfg!(target_os = "windows") {
        "watch-dog-target.exe"
    } else {
        "watch-dog-target"
    };

    // Search release and debug outputs so developer target builds do not need
    // the expensive optimized release link.
    let candidates = [
        dir.join(&file_name),
        dir.join("bin").join(&file_name),
        dir.join("../debug/bin").join(&file_name),
        dir.join("../debug").join(&file_name),
        dir.join("../release/bin").join(&file_name),
        dir.join("../release").join(&file_name),
    ];

    let target_exe = candidates.iter().find(|p| p.exists()).ok_or_else(|| {
        let paths: Vec<String> = candidates.iter().map(|p| p.display().to_string()).collect();
        format!("Target exe not found. Searched: {}", paths.join(", "))
    })?;

    let mut cmd = std::process::Command::new(&target_exe);
    if !label.is_empty() {
        cmd.arg(label);
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to launch target app: {}", e))?;

    log::info!(
        "[node_test] launched target app: {} label='{}' (pid={})",
        target_exe.display(),
        label,
        child.id()
    );
    Ok(child)
}

/// Find or launch the target window, bring it to foreground, and update the
/// frontend capture selection. Node execution requests streaming; the explicit
/// Open Window action only selects the target.
/// Returns the hwnd on success.
async fn resolve_target(
    target: &PipelineTarget,
    app: &tauri::AppHandle,
    start_streaming: bool,
) -> Result<usize, String> {
    let capture_action = if start_streaming {
        "streaming"
    } else {
        "capture"
    };
    if let PipelineTarget::Screen { index } = target {
        let _ = app.emit(
            "update_capture",
            UpdateCapturePayload {
                r#type: "screen".to_string(),
                action: capture_action.to_string(),
                target: target.clone(),
                hwnd: None,
            },
        );
        log::info!(
            "[node_test] selected screen index={}, emitted update_capture",
            index
        );
        return Ok(*index);
    }

    // Try to find the window
    let hwnd = match crate::repositories::window::find_window_by_target(target) {
        Ok(h) => {
            bring_to_front(h);
            h
        }
        Err(_) => {
            // Window not found — try to launch the target app
            log::info!("[node_test] window not found, attempting to launch target app");
            let mut child = launch_target_app(target)?;

            // Log visible windows before polling starts
            crate::repositories::window::log_visible_windows();

            // Poll for the window to appear (up to 10 seconds)
            let mut found = None;
            for i in 0..100 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                // Check if the spawned process is still alive
                match child.try_wait() {
                    Ok(Some(status)) => {
                        log::warn!(
                            "[node_test] target app exited with status: {} after {}ms",
                            status,
                            (i + 1) * 100
                        );
                        crate::repositories::window::log_visible_windows();
                        return Err(format!(
                            "Target app exited with status: {} after launching (searched: {:?})",
                            status, target
                        ));
                    }
                    Ok(None) => { /* still running, continue polling */ }
                    Err(e) => {
                        log::warn!(
                            "[node_test] failed to check target app status: {} after {}ms",
                            e,
                            (i + 1) * 100
                        );
                    }
                }

                if let Ok(h) = crate::repositories::window::find_window_by_target(target) {
                    log::info!("[node_test] window found after {}ms", (i + 1) * 100);
                    found = Some(h);
                    break;
                }

                // Log visible windows periodically (every 2 seconds)
                if (i + 1) % 20 == 0 {
                    log::info!(
                        "[node_test] still waiting for window after {}ms...",
                        (i + 1) * 100
                    );
                    crate::repositories::window::log_visible_windows();
                }
            }
            match found {
                Some(h) => {
                    bring_to_front(h);
                    h
                }
                None => {
                    log::warn!("[node_test] window not found after 10s, listing visible windows:");
                    crate::repositories::window::log_visible_windows();
                    return Err(format!(
                        "Window not found after launching target app: {:?}",
                        target
                    ));
                }
            }
        }
    };

    // Emit update_capture event for frontend
    let capture_type = match target {
        PipelineTarget::Screen { .. } => "screen".to_string(),
        PipelineTarget::Window { .. } => "window".to_string(),
    };
    let hwnd_opt = match target {
        PipelineTarget::Screen { .. } => None,
        PipelineTarget::Window { .. } => Some(hwnd),
    };

    let _ = app.emit(
        "update_capture",
        UpdateCapturePayload {
            r#type: capture_type,
            action: capture_action.to_string(),
            target: target.clone(),
            hwnd: hwnd_opt,
        },
    );

    log::info!(
        "[node_test] resolved target hwnd={}, emitted update_capture",
        hwnd
    );
    Ok(hwnd)
}

fn stream_target_matches(target: &PipelineTarget, status: &StreamStatus, hwnd: usize) -> bool {
    match target {
        PipelineTarget::Screen { index } => {
            status.target_type == "screen" && status.target_index == *index
        }
        PipelineTarget::Window { capture_area, .. } => {
            let expected_area = match capture_area {
                crate::modules::pipeline::models::graph::CaptureArea::Frame => "frame",
                crate::modules::pipeline::models::graph::CaptureArea::Client => "client",
            };
            status.target_type == "window"
                && status.target_index == hwnd
                && status.capture_area == expected_area
        }
    }
}

fn stream_target_for_node(target: &PipelineTarget, hwnd: usize) -> StreamTarget {
    match target {
        PipelineTarget::Screen { index } => StreamTarget::Screen { index: *index },
        PipelineTarget::Window { capture_area, .. } => StreamTarget::Window {
            hwnd,
            capture_area: *capture_area,
        },
    }
}

fn stream_engine_from_status(status: &StreamStatus) -> CaptureEngine {
    match status.engine.as_str() {
        "gdi" => CaptureEngine::Gdi,
        #[cfg(target_os = "windows")]
        "dxgi" => CaptureEngine::Dxgi,
        _ => CaptureEngine::Graphics,
    }
}

#[tauri::command]
pub async fn open_window(target: PipelineTarget, app: tauri::AppHandle) -> Result<String, String> {
    log::info!("[node_test] open_window: {:?}", target);
    let hwnd = resolve_target(&target, &app, false).await?;
    let detail = match target {
        PipelineTarget::Screen { index } => format!("Selected screen index={}", index),
        PipelineTarget::Window { .. } => format!("Opened window hwnd={}", hwnd),
    };
    log::info!("[node_test] {}", detail);
    Ok(detail)
}

#[tauri::command]
pub async fn test_node(
    node: NodeMetaInfo,
    save_debug_images: Option<bool>,
    user_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    detailed_timings: Option<bool>,
    stream_state: tauri::State<'_, super::stream::StreamState>,
    app: tauri::AppHandle,
) -> Result<NodeTestResult, String> {
    let start = std::time::Instant::now();
    let save_debug_images = save_debug_images.unwrap_or(false);
    let detailed_timings = detailed_timings.unwrap_or(false);
    let mut timings = std::collections::BTreeMap::new();
    log::info!("[node_test] test_node '{}' started", node.id);

    if !node.enabled {
        return Ok(NodeTestResult {
            success: false,
            matched: false,
            confidence: 0.0,
            box_region: None,
            actions_fired: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("Node '{}' is disabled", node.id)),
            candidates: Vec::new(),
            selection_error: None,
            screen_click_position: None,
            timings_ms: detailed_timings.then_some(timings),
        });
    }

    let mut resolved_params: std::collections::HashMap<String, serde_json::Value> = node
        .params
        .iter()
        .filter(|param| !param.default.is_null())
        .map(|param| (param.key.clone(), param.default.clone()))
        .collect();
    if let Some(user_params) = user_params {
        resolved_params.extend(user_params);
    }
    let actions = crate::modules::pipeline::engine::runner::substitute_actions(
        &node.actions,
        &resolved_params,
    );

    let target = node.config.target.clone().ok_or_else(|| {
        let msg = format!("Node '{}' has no target window configured", node.id);
        log::warn!("[node_test] {}", msg);
        msg
    })?;

    // Resolve target: find/launch window, bring to foreground, emit update_capture
    let phase_start = std::time::Instant::now();
    let hwnd = resolve_target(&target, &app, true).await?;
    if detailed_timings {
        timings.insert(
            "target_resolve".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }

    // A native HWND can be discoverable before its WebView has rendered. Keep
    // this readiness guard even for an existing window: bringing it forward or
    // navigating between target pages may still leave the next captured frame
    // temporarily incomplete.
    let phase_start = std::time::Instant::now();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    if detailed_timings {
        timings.insert(
            "target_settle".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }
    log::info!("[node_test] waited 2s for target UI content to become capturable");

    let stream_service = stream_state.0.clone();

    // Reuse an exact stream match. If the frontend has a stream for a different
    // capture area (for example, the manual client preference while this node
    // requests frame), restart it here rather than waiting for a frontend event
    // that may not be mounted in the tester entrypoint.
    let status = stream_service.status().await;
    let needs_stream_restart =
        !status.is_streaming || !stream_target_matches(&target, &status, hwnd);

    if needs_stream_restart {
        log::info!(
            "[node_test] restarting stream for node target: current=({}, {}, {}), expected={:?}",
            status.target_type,
            status.target_index,
            status.capture_area,
            stream_target_for_node(&target, hwnd),
        );
        stream_service
            .start(
                app.clone(),
                status.fps.max(1),
                stream_target_for_node(&target, hwnd),
                stream_engine_from_status(&status),
            )
            .await;
    } else {
        log::info!(
            "[node_test] stream already active with correct target hwnd={}",
            hwnd
        );
    }

    // Stream startup may create a native capture session asynchronously.
    let mut waited = false;
    let phase_start = std::time::Instant::now();
    for _ in 0..50 {
        let current_status = stream_service.status().await;
        if current_status.is_streaming && stream_target_matches(&target, &current_status, hwnd) {
            break;
        }
        if !waited {
            log::info!("[node_test] waiting for stream to start...");
            waited = true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let ready_status = stream_service.status().await;
    if detailed_timings {
        timings.insert(
            "stream_ready".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }
    if !ready_status.is_streaming || !stream_target_matches(&target, &ready_status, hwnd) {
        let msg = "Stream not started within 5s".to_string();
        log::warn!("[node_test] {}", msg);
        return Ok(NodeTestResult {
            success: false,
            matched: false,
            confidence: 0.0,
            box_region: None,
            actions_fired: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some(msg),
            candidates: Vec::new(),
            selection_error: None,
            screen_click_position: None,
            timings_ms: detailed_timings.then_some(timings),
        });
    }

    // Subscribe before inspecting the cache so a frame arriving between the
    // cache check and the receive cannot be lost. Windows Graphics Capture may
    // emit only when the target changes, so a late subscriber cannot rely on a
    // future frame for a static test page.
    let mut rx = stream_service.subscribe();
    let expected_stream_target = stream_target_for_node(&target, hwnd);
    let cached_frame = crate::FRAME_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .filter(|frame| frame.target == expected_stream_target);

    let phase_start = std::time::Instant::now();
    let stream_frame = if let Some(frame) = cached_frame {
        log::info!("[node_test] using latest cached stream frame");
        frame
    } else {
        match tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv()).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(e)) => {
                let msg = format!("Stream recv error: {}", e);
                log::warn!("[node_test] {}", msg);
                return Ok(NodeTestResult {
                    success: false,
                    matched: false,
                    confidence: 0.0,
                    box_region: None,
                    actions_fired: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(msg),
                    candidates: Vec::new(),
                    selection_error: None,
                    screen_click_position: None,
                    timings_ms: detailed_timings.then_some(timings),
                });
            }
            Err(_) => {
                let msg = "Stream frame timeout (3s)".to_string();
                log::warn!("[node_test] {}", msg);
                return Ok(NodeTestResult {
                    success: false,
                    matched: false,
                    confidence: 0.0,
                    box_region: None,
                    actions_fired: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(msg),
                    candidates: Vec::new(),
                    selection_error: None,
                    screen_click_position: None,
                    timings_ms: detailed_timings.then_some(timings),
                });
            }
        }
    };
    if detailed_timings {
        timings.insert(
            "frame_acquire".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }

    // Decode image
    let phase_start = std::time::Instant::now();
    let img = image::load_from_memory(&stream_frame.bytes).map_err(|e| {
        let msg = format!("Failed to decode frame: {}", e);
        log::warn!("[node_test] {}", msg);
        msg
    })?;
    if detailed_timings {
        timings.insert(
            "frame_decode".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }

    // Run recognition
    let phase_start = std::time::Instant::now();
    let registry = create_registry();
    let effective_roi =
        crate::modules::pipeline::plugins::shared::resolve_recognition_roi_with_offset(
            node.recognition.roi.as_ref(),
            node.recognition.roi_offset.as_ref(),
            img.width(),
            img.height(),
        )
        .map_err(|error| error.to_string())?;
    let mut match_result = registry
        .evaluate_recognition_with_selection(
            &img,
            &node.recognition.kind,
            Some(&effective_roi),
            node.recognition.candidate_selection.as_ref(),
        )
        .map_err(|e| {
            let msg = format!("Recognition error: {}", e);
            log::warn!("[node_test] {}", msg);
            msg
        })?;
    if detailed_timings {
        timings.insert(
            "recognition".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }
    let debug_match_result = match_result.clone();

    // Convert capture-relative recognition pixels to physical virtual-screen pixels.
    let mut coordinate_error = None;
    let phase_start = std::time::Instant::now();
    if let Some(region) = match_result.box_region.as_mut() {
        let local = region.clone();
        let converted = match stream_frame.bounds {
            Some(bounds) => Ok(crate::modules::pipeline::plugins::shared::map_region(
                &local,
                img.width(),
                img.height(),
                bounds,
            )),
            None => match &target {
                PipelineTarget::Window { capture_area, .. } => {
                    crate::modules::pipeline::plugins::shared::window_region_to_screen(
                        hwnd,
                        *capture_area,
                        &local,
                        img.width(),
                        img.height(),
                    )
                }
                PipelineTarget::Screen { index } => {
                    crate::modules::pipeline::plugins::shared::screen_region_to_screen(
                        *index,
                        &local,
                        img.width(),
                        img.height(),
                    )
                }
            },
        };
        match converted {
            Ok(screen_region) => {
                log::info!(
                    "[node_test] '{}' region conversion: local=({}, {}, {}x{}), frame={}x{}, screen=({}, {}, {}x{}), click=({},{})",
                    node.id, local.x, local.y, local.width, local.height,
                    img.width(), img.height(), screen_region.x, screen_region.y,
                    screen_region.width, screen_region.height,
                    screen_region.center().0, screen_region.center().1,
                );
                *region = screen_region;
            }
            Err(error) => {
                let message = format!("Coordinate conversion failed: {}", error);
                log::warn!("[node_test] '{}': {}", node.id, message);
                coordinate_error = Some(message);
            }
        }
    }
    if detailed_timings {
        timings.insert(
            "coordinate".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }

    let matched =
        crate::modules::pipeline::models::recognizer::effective_match(&match_result, node.inverse);
    let confidence = match_result.confidence;
    let box_region = match_result.box_region.clone();

    if matched {
        log::info!(
            "[node_test] '{}' matched, confidence={:.3}{}",
            node.id,
            confidence,
            match &match_result.text {
                Some(t) => format!(", text=\"{}\"", t),
                None => String::new(),
            }
        );
    } else {
        log::info!(
            "[node_test] '{}' not matched, confidence={:.3}",
            node.id,
            confidence
        );
    }

    // Save debug image
    if save_debug_images {
        let phase_start = std::time::Instant::now();
        let debug_actions = if matched { actions.as_slice() } else { &[] };
        log::info!(
            "[node_test] '{}' saving debug image (matched={})",
            node.id,
            matched
        );
        let mut debug_regions = std::collections::HashMap::new();
        if let Some(region) = debug_match_result.box_region.as_ref() {
            debug_regions.insert(node.id.clone(), region.clone());
        }
        if let Err(e) = debug_image::save_debug_image(
            &img,
            &node.id,
            0,
            &node.recognition.kind,
            Some(&effective_roi),
            &debug_match_result,
            debug_actions,
            &debug_regions,
        ) {
            log::warn!(
                "[node_test] '{}' failed to save debug image: {}",
                node.id,
                e
            );
        }
        if detailed_timings {
            timings.insert(
                "debug_image".to_string(),
                phase_start.elapsed().as_millis() as u64,
            );
        }
    }

    // Fire actions if matched
    let mut actions_fired = 0u32;
    let mut action_error = coordinate_error;

    let phase_start = std::time::Instant::now();
    if matched && action_error.is_none() {
        use std::collections::HashMap;
        let mut recognized_regions = HashMap::new();
        if let Some(region) = &box_region {
            recognized_regions.insert(node.id.clone(), region.clone());
        }

        for action in &actions {
            match registry
                .execute_action(action, box_region.as_ref(), &recognized_regions)
                .await
            {
                Ok(()) => {
                    actions_fired += 1;
                    log::info!("[node_test] '{}' action fired: {:?}", node.id, action);
                }
                Err(e) => {
                    action_error = Some(format!("Action error: {}", e));
                    log::warn!("[node_test] '{}' action error: {}", node.id, e);
                    break;
                }
            }
        }
    }
    if detailed_timings {
        timings.insert(
            "actions".to_string(),
            phase_start.elapsed().as_millis() as u64,
        );
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    log::info!(
        "[node_test] '{}' completed: matched={}, actions={}, duration={}ms",
        node.id,
        matched,
        actions_fired,
        duration_ms
    );
    let screen_click_position = box_region.as_ref().map(|region| {
        let (x, y) = region.center();
        [x, y]
    });

    let success = matched && action_error.is_none() && debug_match_result.selection_error.is_none();
    Ok(NodeTestResult {
        success,
        matched,
        confidence,
        box_region,
        actions_fired,
        duration_ms,
        error: action_error,
        candidates: debug_match_result.candidates,
        selection_error: debug_match_result.selection_error,
        screen_click_position,
        timings_ms: detailed_timings.then_some(timings),
    })
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    use crate::modules::pipeline::models::action::Action;
    use crate::modules::pipeline::models::graph::{CaptureArea, PipelineTarget};
    use crate::modules::pipeline::models::recognizer::{CandidateOrder, Recognition};
    use std::collections::BTreeSet;

    fn window_stream_status(capture_area: &str) -> StreamStatus {
        StreamStatus {
            is_streaming: true,
            fps: 5,
            target_type: "window".to_string(),
            target_index: 123,
            start_time: Some(0),
            engine: "graphics".to_string(),
            capture_area: capture_area.to_string(),
        }
    }

    #[test]
    fn node_stream_match_requires_the_requested_window_capture_area() {
        let frame_target = PipelineTarget::Window {
            process: None,
            class: None,
            title: Some("Target UI".to_string()),
            capture_area: CaptureArea::Frame,
        };
        let client_target = PipelineTarget::Window {
            process: None,
            class: None,
            title: Some("Target UI".to_string()),
            capture_area: CaptureArea::Client,
        };

        assert!(stream_target_matches(
            &frame_target,
            &window_stream_status("frame"),
            123,
        ));
        assert!(!stream_target_matches(
            &frame_target,
            &window_stream_status("client"),
            123,
        ));
        assert!(stream_target_matches(
            &client_target,
            &window_stream_status("client"),
            123,
        ));
    }

    #[test]
    fn node_stream_target_keeps_the_metadata_capture_area() {
        let target = PipelineTarget::Window {
            process: None,
            class: None,
            title: Some("Target UI".to_string()),
            capture_area: CaptureArea::Frame,
        };

        assert_eq!(
            stream_target_for_node(&target, 123),
            StreamTarget::Window {
                hwnd: 123,
                capture_area: CaptureArea::Frame,
            },
        );
    }

    #[test]
    fn node_test_discovers_nodes_from_the_canonical_metadata_shape() {
        let nodes = list_node_metas().unwrap();
        assert!(!nodes.is_empty());
        assert!(nodes
            .iter()
            .all(|node| !node.file_name.ends_with(NODE_META_SUFFIX)));

        let listed = nodes
            .iter()
            .find(|node| node.id == "ClickLogin")
            .expect("ClickLogin node should be discovered");
        assert_eq!(listed.file_name, "ClickLogin");
    }

    #[test]
    fn built_in_nodes_parse_and_cover_registered_engine_features() {
        let directory = paths::built_in_pipeline_nodes_dir();
        let mut recognitions = BTreeSet::new();
        let mut actions = BTreeSet::new();
        let mut candidate_orders = BTreeSet::new();
        let mut has_screen = false;
        let mut has_frame = false;
        let mut has_client = false;
        let mut has_roi = false;
        let mut has_offset = false;
        let mut has_node_reference = false;

        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            let Some(file_name) = node_meta_file_name(&path) else {
                continue;
            };
            let metadata = read_node_metadata(&path).unwrap();
            let (id, config) = metadata.nodes.into_iter().next().unwrap();
            let node = NodeMetaInfo {
                id,
                file_name,
                source: "built-in".to_string(),
                config,
            };

            recognitions.insert(match &node.recognition.kind {
                Recognition::DirectHit => "DirectHit",
                Recognition::TemplateMatch { template, .. } => {
                    image::open(paths::built_in_images_dir().join(template)).unwrap();
                    "TemplateMatch"
                }
                Recognition::Ocr { .. } => "OCR",
                Recognition::ColorMatch { .. } => "ColorMatch",
                Recognition::Custom { .. } => "Custom",
            });
            if let Some(selection) = &node.recognition.candidate_selection {
                candidate_orders.insert(match selection.order {
                    CandidateOrder::Native => "native",
                    CandidateOrder::TopToBottom => "top_to_bottom",
                    CandidateOrder::BottomToTop => "bottom_to_top",
                    CandidateOrder::LeftToRight => "left_to_right",
                    CandidateOrder::RightToLeft => "right_to_left",
                    CandidateOrder::ConfidenceDesc => "confidence_desc",
                });
            }
            match &node.target {
                Some(PipelineTarget::Screen { .. }) => has_screen = true,
                Some(PipelineTarget::Window {
                    capture_area: CaptureArea::Frame,
                    ..
                }) => has_frame = true,
                Some(PipelineTarget::Window {
                    capture_area: CaptureArea::Client,
                    ..
                }) => has_client = true,
                None => {}
            }
            has_roi |= node.recognition.roi.is_some();

            for action in &node.actions {
                actions.insert(match action {
                    Action::DoNothing => "DoNothing",
                    Action::Click {
                        target,
                        target_offset,
                    } => {
                        has_offset |= target_offset.is_some();
                        has_node_reference |= matches!(
                            target,
                            crate::modules::pipeline::models::action::Target::Node(_)
                        );
                        "Click"
                    }
                    Action::LongPress { target_offset, .. } => {
                        has_offset |= target_offset.is_some();
                        "LongPress"
                    }
                    Action::Swipe {
                        begin_offset,
                        end_offset,
                        ..
                    } => {
                        has_offset |= begin_offset.is_some() || end_offset.is_some();
                        "Swipe"
                    }
                    Action::Scroll { target_offset, .. } => {
                        has_offset |= target_offset.is_some();
                        "Scroll"
                    }
                    Action::ClickKey { .. } => "ClickKey",
                    Action::InputText { .. } => "InputText",
                    Action::StopTask => "StopTask",
                    Action::Custom { .. } => "Custom",
                });
            }
        }

        assert_eq!(
            recognitions,
            BTreeSet::from(["ColorMatch", "DirectHit", "OCR", "TemplateMatch"])
        );
        assert_eq!(
            actions,
            BTreeSet::from([
                "Click",
                "ClickKey",
                "DoNothing",
                "InputText",
                "LongPress",
                "Scroll",
                "StopTask",
                "Swipe",
            ])
        );
        assert_eq!(
            candidate_orders,
            BTreeSet::from([
                "bottom_to_top",
                "confidence_desc",
                "left_to_right",
                "native",
                "right_to_left",
                "top_to_bottom",
            ])
        );
        assert!(has_screen && has_frame && has_client && has_roi);
        assert!(has_offset && has_node_reference);
    }
}

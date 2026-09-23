use super::super::RunHandle;
use super::ActiveCapture;
use crate::modules::capture::services::stream::{CaptureEngine, StreamService, StreamTarget};
use tauri::{AppHandle, Emitter};

pub(super) async fn ensure_effective_target(
    graph_target: Option<&crate::modules::pipeline::authoring::models::graph::PipelineTarget>,
    node_target: Option<&crate::modules::pipeline::authoring::models::graph::PipelineTarget>,
    stream_service: &StreamService,
    app: &AppHandle,
    handle: &RunHandle,
    active_capture: &mut Option<ActiveCapture>,
) -> Result<(StreamTarget, u64), String> {
    use crate::modules::pipeline::authoring::models::graph::PipelineTarget;
    let configured = node_target.or(graph_target);
    let configured_key = format!("{configured:?}");
    if let Some(active) = active_capture
        .as_ref()
        .filter(|active| active.configured_key == configured_key && stream_service.is_streaming())
    {
        // The node still uses the same declared target and live capture
        // session. Avoid repeated window enumeration/foreground calls on each
        // recognition retry; action execution restores and verifies foreground
        // ownership immediately before each input action.
        return Ok((active.stream_target.clone(), active.session_id));
    }
    let inherited_window_size = node_target
        .and_then(PipelineTarget::window_size)
        .or_else(|| graph_target.and_then(PipelineTarget::window_size));
    let status = stream_service.status().await;
    let (stream_target, effective_target) = match configured {
        Some(PipelineTarget::Screen { index }) => (
            StreamTarget::Screen { index: *index },
            PipelineTarget::Screen { index: *index },
        ),
        Some(
            target @ PipelineTarget::Window {
                process,
                class,
                title,
                capture_area,
                ..
            },
        ) => {
            let hwnd = if target.has_window_filters() {
                crate::modules::capture::repositories::window::find_window_by_target(target)
                    .map_err(|error| format!("Failed to resolve node window target: {error}"))?
            } else if status.is_streaming && status.target_type == "window" {
                status.target_index
            } else {
                return Err(
                    "Filterless node window target requires an active/default window target"
                        .to_string(),
                );
            };
            (
                StreamTarget::Window {
                    hwnd,
                    capture_area: *capture_area,
                },
                PipelineTarget::Window {
                    process: process.clone(),
                    class: class.clone(),
                    title: title.clone(),
                    capture_area: *capture_area,
                    window_size: inherited_window_size,
                },
            )
        }
        None if status.is_streaming && status.target_type == "screen" => (
            StreamTarget::Screen {
                index: status.target_index,
            },
            PipelineTarget::Screen {
                index: status.target_index,
            },
        ),
        None if status.is_streaming && status.target_type == "window" => {
            let capture_area = if status.capture_area == "client" {
                crate::modules::pipeline::authoring::models::graph::CaptureArea::Client
            } else {
                crate::modules::pipeline::authoring::models::graph::CaptureArea::Frame
            };
            (
                StreamTarget::Window {
                    hwnd: status.target_index,
                    capture_area,
                },
                PipelineTarget::Window {
                    process: None,
                    class: None,
                    title: None,
                    capture_area,
                    window_size: inherited_window_size,
                },
            )
        }
        None => return Err("Pipeline has no effective capture target".to_string()),
    };

    let foreground_changed = match &stream_target {
        StreamTarget::Window { hwnd, .. } => {
            let hwnd = *hwnd;
            tokio::task::spawn_blocking(move || {
                crate::modules::capture::repositories::window::ensure_window_foreground(hwnd)
            })
            .await
            .map_err(|error| format!("Target foreground worker failed: {error}"))??
        }
        StreamTarget::Screen { .. } => false,
    };

    let matches =
        status.is_streaming && match &stream_target {
            StreamTarget::Screen { index } => {
                status.target_type == "screen" && status.target_index == *index
            }
            StreamTarget::Window { hwnd, capture_area } => status.target_type == "window"
                && status.target_index == *hwnd
                && status.capture_area
                    == if *capture_area
                        == crate::modules::pipeline::authoring::models::graph::CaptureArea::Client
                    {
                        "client"
                    } else {
                        "frame"
                    },
        } && status.engine == "gdi" && !foreground_changed;
    if !matches {
        stream_service.stop().await;
        stream_service
            .start(app.clone(), 5, stream_target.clone(), CaptureEngine::Gdi)
            .await?;
        let (kind, hwnd) = match &stream_target {
            StreamTarget::Screen { .. } => ("screen", None),
            StreamTarget::Window { hwnd, .. } => ("window", Some(*hwnd)),
        };
        let _ = app.emit(
            "update_capture",
            serde_json::json!({"type":kind,"action":"sync","target":effective_target,"hwnd":hwnd}),
        );
    }
    if let Ok(mut target) = handle.effective_target.lock() {
        *target = Some(effective_target);
    }
    let session_id = stream_service.status().await.session_id;
    *active_capture = Some(ActiveCapture {
        configured_key,
        stream_target: stream_target.clone(),
        session_id,
    });
    Ok((stream_target, session_id))
}

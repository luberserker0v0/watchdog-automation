use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use crate::modules;
use crate::modules::capture::models::target::CaptureArea;
use crate::modules::capture::services::bounds::{
    screen_capture_bounds, window_capture_bounds, CaptureBounds,
};
use crate::{FRAME_CACHE, STREAM_FRAME};

#[cfg(target_os = "windows")]
use crate::modules::capture::services::capture_dxgi::DxgiCapture;
#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
#[cfg(target_os = "windows")]
use windows_capture::frame::Frame;
#[cfg(target_os = "windows")]
use windows_capture::graphics_capture_api::InternalCaptureControl;
#[cfg(target_os = "windows")]
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureEngine {
    Gdi,
    #[cfg(target_os = "windows")]
    Dxgi,
    #[default]
    Graphics,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamFrame {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub target: StreamTarget,
    pub bounds: Option<CaptureBounds>,
    #[serde(skip)]
    pub captured_at: Instant,
    pub session_id: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamStatus {
    pub is_streaming: bool,
    pub fps: u32,
    pub target_type: String,
    pub target_index: usize,
    pub start_time: Option<u64>,
    pub engine: String,
    pub capture_area: String,
    pub owner: String,
    pub owner_id: Option<String>,
    pub session_id: u64,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
    pub client_width: Option<u32>,
    pub client_height: Option<u32>,
}

static FRAME_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StreamTarget {
    Screen {
        index: usize,
    },
    Window {
        hwnd: usize,
        #[serde(default)]
        capture_area: CaptureArea,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
struct StreamStoppedPayload {
    reason: String,
    target_type: String,
    target_index: usize,
    session_id: u64,
}

fn stream_target_identity(target: &StreamTarget) -> (&'static str, usize) {
    match target {
        StreamTarget::Screen { index } => ("screen", *index),
        StreamTarget::Window { hwnd, .. } => ("window", *hwnd),
    }
}

#[cfg(target_os = "windows")]
fn current_window_client_size(hwnd: usize) -> Option<[u32; 2]> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let hwnd = HWND(hwnd as isize as *mut c_void);
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect).ok()? };
    Some([
        (rect.right - rect.left).max(0) as u32,
        (rect.bottom - rect.top).max(0) as u32,
    ])
}

#[cfg(not(target_os = "windows"))]
fn current_window_client_size(_hwnd: usize) -> Option<[u32; 2]> {
    None
}

fn emit_frame_available(app: &AppHandle, frame: &StreamFrame) {
    let (target_type, target_index) = stream_target_identity(&frame.target);
    let capture_area = match &frame.target {
        StreamTarget::Window {
            capture_area: CaptureArea::Client,
            ..
        } => "client",
        _ => "frame",
    };
    let client_size = match &frame.target {
        StreamTarget::Window { hwnd, .. } => current_window_client_size(*hwnd),
        StreamTarget::Screen { .. } => None,
    };
    let _ = app.emit(
        "stream-frame",
        serde_json::json!({
            "session_id": frame.session_id,
            "sequence": frame.sequence,
            "target_type": target_type,
            "target_index": target_index,
            "capture_area": capture_area,
            "frame_width": frame.width,
            "frame_height": frame.height,
            "client_width": client_size.map(|size| size[0]),
            "client_height": client_size.map(|size| size[1]),
        }),
    );
}

fn finish_stream_session(
    is_streaming: &AtomicBool,
    session_generation: &AtomicU64,
    session_id: u64,
    app: &AppHandle,
    target: &StreamTarget,
    reason: &str,
) {
    if session_generation.load(Ordering::Acquire) != session_id
        || !is_streaming.swap(false, Ordering::AcqRel)
    {
        return;
    }

    *STREAM_FRAME.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let (target_type, target_index) = stream_target_identity(target);
    let _ = app.emit(
        "stream-stopped",
        StreamStoppedPayload {
            reason: reason.to_string(),
            target_type: target_type.to_string(),
            target_index,
            session_id,
        },
    );
    log::info!("Stream ended: reason={}, target={:?}", reason, target);
}

#[cfg(target_os = "windows")]
fn stream_target_is_available(target: &StreamTarget) -> bool {
    match target {
        StreamTarget::Screen { .. } => true,
        StreamTarget::Window { hwnd, .. } => unsafe {
            use windows::Win32::UI::WindowsAndMessaging::IsWindow;
            IsWindow(Some(HWND(*hwnd as isize as *mut c_void))).as_bool()
        },
    }
}

#[cfg(not(target_os = "windows"))]
fn stream_target_is_available(target: &StreamTarget) -> bool {
    match target {
        StreamTarget::Screen { index } => modules::capture::services::capture::get_screens()
            .map(|screens| *index < screens.len())
            .unwrap_or(false),
        StreamTarget::Window { hwnd, .. } => {
            modules::capture::services::capture::get_windows(None, None)
                .map(|windows| windows.iter().any(|window| window.hwnd == Some(*hwnd)))
                .unwrap_or(false)
        }
    }
}

fn resolve_frame_bounds(
    target: &StreamTarget,
    frame_width: u32,
    frame_height: u32,
) -> Option<CaptureBounds> {
    let result = match target {
        StreamTarget::Screen { index } => screen_capture_bounds(*index),
        StreamTarget::Window { hwnd, capture_area } => {
            window_capture_bounds(*hwnd, *capture_area, frame_width, frame_height)
        }
    };
    result
        .map_err(|error| log::warn!("Unable to cache frame coordinate bounds: {}", error))
        .ok()
}

// ===== Windows-specific CaptureHandler =====

#[cfg(target_os = "windows")]
struct CaptureFlags {
    sender: broadcast::Sender<StreamFrame>,
    app: AppHandle,
    target: StreamTarget,
    is_streaming: Arc<AtomicBool>,
    session_generation: Arc<AtomicU64>,
    session_id: u64,
}

#[cfg(target_os = "windows")]
struct CaptureHandler {
    sender: broadcast::Sender<StreamFrame>,
    app: AppHandle,
    raw_buffer: Vec<u8>,
    crop_buffer: Vec<u8>,
    target: StreamTarget,
    is_streaming: Arc<AtomicBool>,
    session_generation: Arc<AtomicU64>,
    session_id: u64,
}

#[cfg(target_os = "windows")]
fn window_client_crop(
    hwnd: usize,
    frame_width: u32,
    frame_height: u32,
) -> Result<(usize, usize, usize, usize), String> {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    unsafe {
        let hwnd = HWND(hwnd as isize as *mut c_void);
        let mut frame_rect = RECT::default();
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame_rect as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
        .map_err(|e| format!("DwmGetWindowAttribute failed: {}", e))?;
        let mut client_rect = RECT::default();
        GetClientRect(hwnd, &mut client_rect)
            .map_err(|e| format!("GetClientRect failed: {}", e))?;
        let mut client_origin = POINT::default();
        if !ClientToScreen(hwnd, &mut client_origin).as_bool() {
            return Err("ClientToScreen failed".to_string());
        }

        let source_width = (frame_rect.right - frame_rect.left).max(1) as f64;
        let source_height = (frame_rect.bottom - frame_rect.top).max(1) as f64;
        let scale_x = frame_width as f64 / source_width;
        let scale_y = frame_height as f64 / source_height;
        let x = ((client_origin.x - frame_rect.left) as f64 * scale_x)
            .round()
            .max(0.0) as usize;
        let y = ((client_origin.y - frame_rect.top) as f64 * scale_y)
            .round()
            .max(0.0) as usize;
        let width = ((client_rect.right - client_rect.left) as f64 * scale_x)
            .round()
            .max(1.0) as usize;
        let height = ((client_rect.bottom - client_rect.top) as f64 * scale_y)
            .round()
            .max(1.0) as usize;
        Ok((
            x.min(frame_width.saturating_sub(1) as usize),
            y.min(frame_height.saturating_sub(1) as usize),
            width.min(frame_width as usize - x.min(frame_width.saturating_sub(1) as usize)),
            height.min(frame_height as usize - y.min(frame_height.saturating_sub(1) as usize)),
        ))
    }
}

#[cfg(target_os = "windows")]
fn dxgi_capture_setup(
    target: &StreamTarget,
) -> Result<
    (
        windows_capture::monitor::Monitor,
        Option<(usize, usize, usize, usize)>,
    ),
    String,
> {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, HMONITOR, MONITORINFO};
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    match target {
        StreamTarget::Screen { index } => windows_capture::monitor::Monitor::from_index(index + 1)
            .map(|monitor| (monitor, None))
            .map_err(|e| format!("DXGI screen {} was not found: {}", index, e)),
        StreamTarget::Window { hwnd, capture_area } => unsafe {
            let raw_hwnd = HWND(*hwnd as isize as *mut c_void);
            let window = windows_capture::window::Window::from_raw_hwnd(raw_hwnd.0);
            let monitor = window
                .monitor()
                .ok_or_else(|| "Target window is not on an active monitor".to_string())?;
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(HMONITOR(monitor.as_raw_hmonitor()), &mut monitor_info).as_bool() {
                return Err("GetMonitorInfoW failed".to_string());
            }

            let bounds = match capture_area {
                CaptureArea::Frame => {
                    let mut rect = RECT::default();
                    DwmGetWindowAttribute(
                        raw_hwnd,
                        DWMWA_EXTENDED_FRAME_BOUNDS,
                        &mut rect as *mut RECT as *mut c_void,
                        std::mem::size_of::<RECT>() as u32,
                    )
                    .map_err(|e| format!("DWM frame bounds failed: {}", e))?;
                    rect
                }
                CaptureArea::Client => {
                    let mut rect = RECT::default();
                    GetClientRect(raw_hwnd, &mut rect)
                        .map_err(|e| format!("GetClientRect failed: {}", e))?;
                    let mut origin = POINT::default();
                    if !ClientToScreen(raw_hwnd, &mut origin).as_bool() {
                        return Err("ClientToScreen failed".to_string());
                    }
                    RECT {
                        left: origin.x,
                        top: origin.y,
                        right: origin.x + rect.right - rect.left,
                        bottom: origin.y + rect.bottom - rect.top,
                    }
                }
            };

            let monitor_rect = monitor_info.rcMonitor;
            let left = bounds.left.max(monitor_rect.left);
            let top = bounds.top.max(monitor_rect.top);
            let right = bounds.right.min(monitor_rect.right);
            let bottom = bounds.bottom.min(monitor_rect.bottom);
            if right <= left || bottom <= top {
                return Err("Window does not intersect its selected monitor".to_string());
            }
            Ok((
                monitor,
                Some((
                    (left - monitor_rect.left) as usize,
                    (top - monitor_rect.top) as usize,
                    (right - left) as usize,
                    (bottom - top) as usize,
                )),
            ))
        },
    }
}

#[cfg(target_os = "windows")]
impl GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = CaptureFlags;
    type Error = String;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            sender: ctx.flags.sender,
            app: ctx.flags.app,
            raw_buffer: Vec::with_capacity(1920 * 1080 * 4),
            crop_buffer: Vec::with_capacity(1920 * 1080 * 4),
            target: ctx.flags.target,
            is_streaming: ctx.flags.is_streaming,
            session_generation: ctx.flags.session_generation,
            session_id: ctx.flags.session_id,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let captured_at = Instant::now();
        let width = frame.width();
        let height = frame.height();

        let frame_buffer = frame.buffer().map_err(|e| e.to_string())?;

        self.raw_buffer.clear();
        let bgra = frame_buffer.as_nopadding_buffer(&mut self.raw_buffer);

        let source_w = width as usize;
        let source_h = height as usize;
        let (crop_x, crop_y, w, h) = match &self.target {
            StreamTarget::Window {
                hwnd,
                capture_area: CaptureArea::Client,
            } => match window_client_crop(*hwnd, width, height) {
                Ok(crop) => crop,
                Err(error) => {
                    log::warn!(
                        "Unable to resolve client crop; using the full frame: {}",
                        error
                    );
                    (0, 0, source_w, source_h)
                }
            },
            _ => (0, 0, source_w, source_h),
        };

        // A cropped sub-image cannot safely retain the source pitch: the final
        // row may not contain a complete source-width stride after its x offset.
        // Copy it to a tightly packed buffer before passing it to libjpeg-turbo.
        let is_full_frame = crop_x == 0 && crop_y == 0 && w == source_w && h == source_h;
        let (pixels, pitch) = if is_full_frame {
            (bgra, source_w * 4)
        } else {
            self.crop_buffer.clear();
            self.crop_buffer.reserve(w * h * 4);
            for row in crop_y..crop_y + h {
                let start = (row * source_w + crop_x) * 4;
                self.crop_buffer
                    .extend_from_slice(&bgra[start..start + w * 4]);
            }
            (&self.crop_buffer[..], w * 4)
        };

        let image = turbojpeg::Image {
            pixels,
            width: w,
            pitch,
            height: h,
            format: turbojpeg::PixelFormat::BGRA,
        };

        let mut compressor = turbojpeg::Compressor::new().map_err(|e| e.to_string())?;
        let _ = compressor.set_quality(85);
        let _ = compressor.set_subsamp(turbojpeg::Subsamp::None);

        let jpeg = compressor
            .compress_to_vec(image)
            .map_err(|e| e.to_string())?;
        let jpeg_bytes = jpeg.to_vec();

        *STREAM_FRAME.lock().map_err(|e| e.to_string())? = Some(jpeg_bytes.clone());
        let stream_frame = StreamFrame {
            bytes: jpeg_bytes,
            width: w as u32,
            height: h as u32,
            target: self.target.clone(),
            bounds: resolve_frame_bounds(&self.target, w as u32, h as u32),
            captured_at,
            session_id: self.session_id,
            sequence: FRAME_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1,
        };
        *FRAME_CACHE.lock().map_err(|e| e.to_string())? = Some(stream_frame.clone());
        emit_frame_available(&self.app, &stream_frame);
        let _ = self.sender.send(stream_frame);

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let reason = match &self.target {
            StreamTarget::Window { .. } => "target_closed",
            StreamTarget::Screen { .. } => "capture_closed",
        };
        finish_stream_session(
            &self.is_streaming,
            &self.session_generation,
            self.session_id,
            &self.app,
            &self.target,
            reason,
        );
        Ok(())
    }
}

// ===== StreamService =====

pub struct StreamService {
    is_streaming: Arc<AtomicBool>,
    session_generation: Arc<AtomicU64>,
    handle: Mutex<Option<JoinHandle<()>>>,
    #[cfg(target_os = "windows")]
    capture_control: Mutex<Option<CaptureControl<CaptureHandler, String>>>,
    fps: Arc<Mutex<u32>>,
    target: Arc<Mutex<Option<StreamTarget>>>,
    engine: Arc<Mutex<CaptureEngine>>,
    sender: broadcast::Sender<StreamFrame>,
    start_time: Arc<Mutex<Option<tokio::time::Instant>>>,
    owner: Mutex<(String, Option<String>)>,
}

impl StreamService {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(4);
        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            session_generation: Arc::new(AtomicU64::new(0)),
            handle: Mutex::new(None),
            #[cfg(target_os = "windows")]
            capture_control: Mutex::new(None),
            fps: Arc::new(Mutex::new(5)),
            target: Arc::new(Mutex::new(None)),
            engine: Arc::new(Mutex::new(CaptureEngine::Graphics)),
            sender,
            start_time: Arc::new(Mutex::new(None)),
            owner: Mutex::new(("none".to_string(), None)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StreamFrame> {
        self.sender.subscribe()
    }

    /// Return the newest cached frame for one capture session/target. Pipeline
    /// runners use this before awaiting the broadcast channel so recognition
    /// retries do not consume a backlog of frames produced before their fresh
    /// frame boundary.
    pub fn latest_frame_after(
        &self,
        session_id: u64,
        target: &StreamTarget,
        min_sequence: Option<u64>,
        not_before: Option<Instant>,
    ) -> Option<StreamFrame> {
        FRAME_CACHE
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .filter(|frame| {
                frame.session_id == session_id
                    && &frame.target == target
                    && min_sequence.map_or(true, |sequence| frame.sequence > sequence)
                    && not_before.map_or(true, |boundary| frame.captured_at >= boundary)
            })
            .cloned()
    }

    pub fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }

    pub async fn reject_if_pipeline_owned(&self) -> Result<(), String> {
        let owner = self.owner.lock().await;
        if owner.0 == "pipeline" {
            return Err(format!(
                "Capture is owned by pipeline '{}'; stop it before using this tool",
                owner.1.as_deref().unwrap_or("unknown")
            ));
        }
        Ok(())
    }

    pub async fn acquire_pipeline_owner(&self, run_id: &str) -> Result<(), String> {
        let mut owner = self.owner.lock().await;
        if owner.0 == "manual" {
            return Err(
                "Capture is owned by manual streaming; stop it before starting a pipeline"
                    .to_string(),
            );
        }
        if owner.0 == "pipeline" {
            return Err(format!(
                "Capture is owned by pipeline '{}'; stop it before starting another pipeline",
                owner.1.as_deref().unwrap_or("unknown")
            ));
        }
        *owner = ("pipeline".to_string(), Some(run_id.to_string()));
        Ok(())
    }

    pub async fn acquire_manual_owner(&self) -> Result<(), String> {
        let mut owner = self.owner.lock().await;
        if owner.0 == "pipeline" {
            return Err(format!(
                "Capture is owned by pipeline '{}'; stop it before using this tool",
                owner.1.as_deref().unwrap_or("unknown")
            ));
        }
        *owner = ("manual".to_string(), None);
        Ok(())
    }

    pub async fn set_owner(&self, kind: &str, id: Option<String>) {
        *self.owner.lock().await = (kind.to_string(), id);
    }

    pub async fn release_owner(&self, kind: &str, id: Option<&str>) {
        let mut owner = self.owner.lock().await;
        if owner.0 == kind && id.map_or(true, |value| owner.1.as_deref() == Some(value)) {
            *owner = ("none".to_string(), None);
        }
    }

    pub async fn start(
        &self,
        app: AppHandle,
        fps: u32,
        target: StreamTarget,
        engine: CaptureEngine,
    ) -> Result<(), String> {
        if self.is_streaming.load(Ordering::Relaxed) {
            self.stop().await;
        }

        let log_target = target.clone();
        let log_engine = engine.clone();
        let session_id = self.session_generation.fetch_add(1, Ordering::AcqRel) + 1;

        *self.fps.lock().await = fps;
        *self.target.lock().await = Some(target.clone());
        *self.engine.lock().await = engine.clone();
        *self.start_time.lock().await = Some(tokio::time::Instant::now());
        self.is_streaming.store(true, Ordering::Relaxed);

        let start_result = match engine {
            #[cfg(target_os = "windows")]
            CaptureEngine::Graphics => self.start_graphics(app, target, session_id).await,
            #[cfg(target_os = "windows")]
            CaptureEngine::Dxgi => self.start_dxgi(app, fps, target, session_id).await,
            CaptureEngine::Gdi => self.start_gdi(app, fps, target, session_id).await,
            #[cfg(not(target_os = "windows"))]
            _ => self.start_gdi(app, fps, target, session_id).await,
        };
        if let Err(error) = start_result {
            self.is_streaming.store(false, Ordering::Relaxed);
            *self.target.lock().await = None;
            *self.start_time.lock().await = None;
            return Err(error);
        }

        log::info!(
            "Stream started: fps={}, target={:?}, engine={:?}",
            fps,
            log_target,
            log_engine
        );
        Ok(())
    }

    // ===== Windows-only: Graphics Capture =====

    #[cfg(target_os = "windows")]
    async fn start_graphics(
        &self,
        app: AppHandle,
        target: StreamTarget,
        session_id: u64,
    ) -> Result<(), String> {
        let sender = self.sender.clone();
        let start_time = self.start_time.clone();
        let is_streaming = self.is_streaming.clone();
        let session_generation = self.session_generation.clone();

        match target {
            StreamTarget::Screen { index } => {
                let settings_result = tokio::task::spawn_blocking(move || {
                    let monitor = windows_capture::monitor::Monitor::from_index(index + 1)
                        .map_err(|e| format!("Failed to get monitor: {}", e))?;
                    Ok::<_, String>(Settings::new(
                        monitor,
                        CursorCaptureSettings::WithoutCursor,
                        DrawBorderSettings::WithoutBorder,
                        SecondaryWindowSettings::Default,
                        MinimumUpdateIntervalSettings::Default,
                        DirtyRegionSettings::Default,
                        ColorFormat::Bgra8,
                        CaptureFlags {
                            sender: sender.clone(),
                            app,
                            target: StreamTarget::Screen { index },
                            is_streaming,
                            session_generation,
                            session_id,
                        },
                    ))
                })
                .await;

                let settings = match settings_result {
                    Ok(Ok(s)) => s,
                    Ok(Err(e)) => {
                        log::error!("Failed to create capture settings: {}", e);
                        *start_time.lock().await = None;
                        self.is_streaming.store(false, Ordering::Relaxed);
                        return Err(e);
                    }
                    Err(e) => {
                        log::error!("Capture settings thread panicked: {}", e);
                        *start_time.lock().await = None;
                        self.is_streaming.store(false, Ordering::Relaxed);
                        return Err(format!("Capture settings thread panicked: {e}"));
                    }
                };

                match CaptureHandler::start_free_threaded(settings) {
                    Ok(control) => {
                        log::info!("Graphics capture started");
                        *self.capture_control.lock().await = Some(control);
                    }
                    Err(e) => {
                        log::error!("Failed to start graphics capture: {}", e);
                        *start_time.lock().await = None;
                        self.is_streaming.store(false, Ordering::Relaxed);
                        return Err(format!("Failed to start graphics capture: {e}"));
                    }
                }
            }
            StreamTarget::Window { hwnd, capture_area } => {
                let settings_result = tokio::task::spawn_blocking(move || {
                    let raw_hwnd = HWND(hwnd as isize as *mut c_void);
                    let window = windows_capture::window::Window::from_raw_hwnd(raw_hwnd.0);
                    Ok::<_, String>(Settings::new(
                        window,
                        CursorCaptureSettings::WithoutCursor,
                        DrawBorderSettings::WithoutBorder,
                        SecondaryWindowSettings::Default,
                        MinimumUpdateIntervalSettings::Default,
                        DirtyRegionSettings::Default,
                        ColorFormat::Bgra8,
                        CaptureFlags {
                            sender: sender.clone(),
                            app,
                            target: StreamTarget::Window { hwnd, capture_area },
                            is_streaming,
                            session_generation,
                            session_id,
                        },
                    ))
                })
                .await;

                let settings = match settings_result {
                    Ok(Ok(s)) => s,
                    Ok(Err(e)) => {
                        log::error!("Failed to create capture settings: {}", e);
                        *start_time.lock().await = None;
                        self.is_streaming.store(false, Ordering::Relaxed);
                        return Err(e);
                    }
                    Err(e) => {
                        log::error!("Capture settings thread panicked: {}", e);
                        *start_time.lock().await = None;
                        self.is_streaming.store(false, Ordering::Relaxed);
                        return Err(format!("Capture settings thread panicked: {e}"));
                    }
                };

                match CaptureHandler::start_free_threaded(settings) {
                    Ok(control) => {
                        log::info!("Graphics capture started");
                        *self.capture_control.lock().await = Some(control);
                    }
                    Err(e) => {
                        log::error!("Failed to start graphics capture: {}", e);
                        *start_time.lock().await = None;
                        self.is_streaming.store(false, Ordering::Relaxed);
                        return Err(format!("Failed to start graphics capture: {e}"));
                    }
                }
            }
        }
        Ok(())
    }

    // ===== Windows-only: DXGI =====

    #[cfg(target_os = "windows")]
    async fn start_dxgi(
        &self,
        app: AppHandle,
        fps: u32,
        target: StreamTarget,
        session_id: u64,
    ) -> Result<(), String> {
        let is_streaming = self.is_streaming.clone();
        let session_generation = self.session_generation.clone();
        let sender = self.sender.clone();
        let target_clone = target.clone();
        let start_time = self.start_time.clone();

        let (monitor, crop) = match dxgi_capture_setup(&target_clone) {
            Ok(setup) => setup,
            Err(error) => {
                log::error!("Failed to prepare DXGI capture: {}", error);
                *start_time.lock().await = None;
                self.is_streaming.store(false, Ordering::Relaxed);
                return Err(format!("Failed to prepare DXGI capture: {error}"));
            }
        };

        let handle = tokio::spawn(async move {
            let interval_ms = 1000 / fps.max(1);
            let timeout_ms = 1000 / fps.max(1);
            let mut dxgi_capture = DxgiCapture::new(monitor);

            loop {
                if !is_streaming.load(Ordering::Relaxed) {
                    break;
                }
                if !stream_target_is_available(&target_clone) {
                    finish_stream_session(
                        &is_streaming,
                        &session_generation,
                        session_id,
                        &app,
                        &target_clone,
                        "target_closed",
                    );
                    break;
                }

                let captured_at = Instant::now();
                match dxgi_capture.capture_jpeg(timeout_ms, crop) {
                    Ok(buf) => {
                        let (width, height) = image::load_from_memory(&buf)
                            .map(|image| (image.width(), image.height()))
                            .unwrap_or((0, 0));
                        *STREAM_FRAME.lock().unwrap_or_else(|e| e.into_inner()) = Some(buf.clone());
                        let stream_frame = StreamFrame {
                            bytes: buf,
                            width,
                            height,
                            target: target_clone.clone(),
                            bounds: resolve_frame_bounds(&target_clone, width, height),
                            captured_at,
                            session_id,
                            sequence: FRAME_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1,
                        };
                        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(stream_frame.clone());
                        emit_frame_available(&app, &stream_frame);
                        let _ = sender.send(stream_frame);
                    }
                    Err(e) => {
                        log::warn!("DXGI stream capture failed: {}", e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(interval_ms.into())).await;
            }

            *start_time.lock().await = None;
        });

        *self.handle.lock().await = Some(handle);
        Ok(())
    }

    // ===== Cross-platform: GDI / xcap polling =====

    async fn start_gdi(
        &self,
        app: AppHandle,
        fps: u32,
        target: StreamTarget,
        session_id: u64,
    ) -> Result<(), String> {
        let is_streaming = self.is_streaming.clone();
        let session_generation = self.session_generation.clone();
        let sender = self.sender.clone();
        let target_clone = target.clone();
        let start_time = self.start_time.clone();

        let (width, height) = match &target_clone {
            StreamTarget::Screen { index } => {
                match modules::capture::services::capture::get_screens() {
                    Ok(screens) => {
                        if let Some(s) = screens.get(*index) {
                            (s.width, s.height)
                        } else {
                            (0, 0)
                        }
                    }
                    Err(_) => (0, 0),
                }
            }
            StreamTarget::Window { hwnd, .. } => {
                match modules::capture::services::capture::get_windows(None, None) {
                    Ok(windows) => {
                        if let Some(w) = windows.iter().find(|w| w.hwnd == Some(*hwnd)) {
                            (w.width, w.height)
                        } else {
                            (0, 0)
                        }
                    }
                    Err(_) => (0, 0),
                }
            }
        };

        let handle = tokio::spawn(async move {
            let interval_ms = 1000 / fps.max(1);

            loop {
                if !is_streaming.load(Ordering::Relaxed) {
                    break;
                }
                if !stream_target_is_available(&target_clone) {
                    finish_stream_session(
                        &is_streaming,
                        &session_generation,
                        session_id,
                        &app,
                        &target_clone,
                        "target_closed",
                    );
                    break;
                }

                let target = target_clone.clone();
                let captured_at = Instant::now();
                let result = tokio::task::spawn_blocking(move || match &target {
                    StreamTarget::Screen { index } => {
                        modules::capture::services::capture::capture_screen_jpeg(*index)
                    }
                    StreamTarget::Window { hwnd, capture_area } => {
                        modules::capture::services::capture::capture_window_jpeg_by_hwnd(
                            *hwnd,
                            *capture_area,
                        )
                    }
                })
                .await
                .unwrap_or_else(|e| {
                    Err(crate::core::error::CaptureError::CaptureFailed(
                        e.to_string(),
                    ))
                });

                match result {
                    Ok(buf) => {
                        let (actual_width, actual_height) = image::load_from_memory(&buf)
                            .map(|image| (image.width(), image.height()))
                            .unwrap_or((width, height));
                        *STREAM_FRAME.lock().unwrap_or_else(|e| e.into_inner()) = Some(buf.clone());
                        let stream_frame = StreamFrame {
                            bytes: buf,
                            width: actual_width,
                            height: actual_height,
                            target: target_clone.clone(),
                            bounds: resolve_frame_bounds(
                                &target_clone,
                                actual_width,
                                actual_height,
                            ),
                            captured_at,
                            session_id,
                            sequence: FRAME_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1,
                        };
                        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(stream_frame.clone());
                        emit_frame_available(&app, &stream_frame);
                        let _ = sender.send(stream_frame);
                    }
                    Err(e) => {
                        log::warn!("Stream capture failed: {}", e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_millis(interval_ms.into())).await;
            }

            *start_time.lock().await = None;
        });

        *self.handle.lock().await = Some(handle);
        Ok(())
    }

    // ===== Shared: stop/status =====

    pub async fn stop(&self) {
        self.session_generation.fetch_add(1, Ordering::AcqRel);
        self.is_streaming.store(false, Ordering::Relaxed);

        #[cfg(target_os = "windows")]
        if let Some(control) = self.capture_control.lock().await.take() {
            let _ = control.stop();
        }

        if let Some(handle) = self.handle.lock().await.take() {
            handle.abort();
        }

        *self.target.lock().await = None;
        *self.start_time.lock().await = None;
        *STREAM_FRAME.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        log::info!("Stream stopped");
    }

    pub async fn stop_and_emit(&self, app: &AppHandle, reason: &str) {
        let was_streaming = self.is_streaming();
        let target = self.target.lock().await.clone();
        let session_id = self.session_generation.load(Ordering::Acquire);
        self.stop().await;
        if was_streaming {
            if let Some(target) = target {
                let (target_type, target_index) = stream_target_identity(&target);
                let _ = app.emit(
                    "stream-stopped",
                    StreamStoppedPayload {
                        reason: reason.to_string(),
                        target_type: target_type.to_string(),
                        target_index,
                        session_id,
                    },
                );
            }
        }
    }

    pub fn stop_sync(&self) {
        self.session_generation.fetch_add(1, Ordering::AcqRel);
        self.is_streaming.store(false, Ordering::Relaxed);

        #[cfg(target_os = "windows")]
        if let Ok(mut guard) = self.capture_control.try_lock() {
            if let Some(control) = guard.take() {
                let _ = control.stop();
            }
        }

        if let Ok(mut guard) = self.handle.try_lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }

        if let Ok(mut guard) = self.target.try_lock() {
            *guard = None;
        }

        if let Ok(mut guard) = self.start_time.try_lock() {
            *guard = None;
        }

        *STREAM_FRAME.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        log::info!("Stream stopped (sync)");
    }

    pub async fn status(&self) -> StreamStatus {
        let is_streaming = self.is_streaming.load(Ordering::Relaxed);
        let target = self.target.lock().await.clone();
        let fps = *self.fps.lock().await;
        let start_time = *self.start_time.lock().await;
        let engine = self.engine.lock().await.clone();
        let owner = self.owner.lock().await.clone();
        let session_id = self.session_generation.load(Ordering::Acquire);

        let (target_type, target_index, capture_area) = match &target {
            Some(StreamTarget::Screen { index }) => {
                ("screen".to_string(), *index, "frame".to_string())
            }
            Some(StreamTarget::Window { hwnd, capture_area }) => (
                "window".to_string(),
                *hwnd,
                match capture_area {
                    CaptureArea::Frame => "frame",
                    CaptureArea::Client => "client",
                }
                .to_string(),
            ),
            None => ("none".to_string(), 0, "frame".to_string()),
        };

        let start_time_ms = is_streaming
            .then(|| start_time.map(|started_at| started_at.elapsed().as_millis() as u64))
            .flatten();
        let engine_str = match engine {
            CaptureEngine::Gdi => "gdi".to_string(),
            #[cfg(target_os = "windows")]
            CaptureEngine::Dxgi => "dxgi".to_string(),
            CaptureEngine::Graphics => "graphics".to_string(),
        };
        let cached_frame = FRAME_CACHE
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .filter(|frame| {
                frame.session_id == session_id && Some(&frame.target) == target.as_ref()
            })
            .map(|frame| (frame.width, frame.height));
        let client_size = match &target {
            Some(StreamTarget::Window { hwnd, .. }) => current_window_client_size(*hwnd),
            _ => None,
        };

        StreamStatus {
            is_streaming,
            fps,
            target_type,
            target_index,
            start_time: start_time_ms,
            engine: engine_str,
            capture_area,
            owner: owner.0,
            owner_id: owner.1,
            session_id,
            frame_width: cached_frame.map(|size| size.0),
            frame_height: cached_frame.map(|size| size.1),
            client_width: client_size.map(|size| size[0]),
            client_height: client_size.map(|size| size[1]),
        }
    }
}

impl Default for StreamService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::StreamService;

    #[tokio::test]
    async fn manual_capture_blocks_pipeline_until_released() {
        let service = StreamService::new();
        service.acquire_manual_owner().await.unwrap();
        assert!(service.acquire_pipeline_owner("run-1").await.is_err());
        assert_eq!(service.status().await.owner, "manual");

        service.release_owner("manual", None).await;
        service.acquire_pipeline_owner("run-1").await.unwrap();
        assert_eq!(service.status().await.owner_id.as_deref(), Some("run-1"));
    }
}

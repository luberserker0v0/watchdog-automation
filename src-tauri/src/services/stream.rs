use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use crate::modules::pipeline::models::graph::CaptureArea;
use crate::modules::pipeline::plugins::shared::CaptureBounds;
use crate::services;
use crate::{FRAME_CACHE, STREAM_FRAME};

#[cfg(target_os = "windows")]
use crate::services::capture_dxgi::DxgiCapture;
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
}

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

fn resolve_frame_bounds(
    target: &StreamTarget,
    frame_width: u32,
    frame_height: u32,
) -> Option<CaptureBounds> {
    let result = match target {
        StreamTarget::Screen { index } => {
            crate::modules::pipeline::plugins::shared::screen_capture_bounds(*index)
        }
        StreamTarget::Window { hwnd, capture_area } => {
            crate::modules::pipeline::plugins::shared::window_capture_bounds(
                *hwnd,
                *capture_area,
                frame_width,
                frame_height,
            )
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
}

#[cfg(target_os = "windows")]
struct CaptureHandler {
    sender: broadcast::Sender<StreamFrame>,
    app: AppHandle,
    raw_buffer: Vec<u8>,
    crop_buffer: Vec<u8>,
    target: StreamTarget,
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
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
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
        };
        *FRAME_CACHE.lock().map_err(|e| e.to_string())? = Some(stream_frame.clone());
        let _ = self.sender.send(stream_frame);
        let _ = self.app.emit("stream-frame", ());

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        log::info!("Graphics capture session ended");
        Ok(())
    }
}

// ===== StreamService =====

pub struct StreamService {
    is_streaming: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
    #[cfg(target_os = "windows")]
    capture_control: Mutex<Option<CaptureControl<CaptureHandler, String>>>,
    fps: Arc<Mutex<u32>>,
    target: Arc<Mutex<Option<StreamTarget>>>,
    engine: Arc<Mutex<CaptureEngine>>,
    sender: broadcast::Sender<StreamFrame>,
    start_time: Arc<Mutex<Option<tokio::time::Instant>>>,
}

impl StreamService {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(4);
        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            handle: Mutex::new(None),
            #[cfg(target_os = "windows")]
            capture_control: Mutex::new(None),
            fps: Arc::new(Mutex::new(5)),
            target: Arc::new(Mutex::new(None)),
            engine: Arc::new(Mutex::new(CaptureEngine::Graphics)),
            sender,
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StreamFrame> {
        self.sender.subscribe()
    }

    pub fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }

    pub async fn start(
        &self,
        app: AppHandle,
        fps: u32,
        target: StreamTarget,
        engine: CaptureEngine,
    ) {
        if self.is_streaming.load(Ordering::Relaxed) {
            self.stop().await;
        }

        let log_target = target.clone();
        let log_engine = engine.clone();

        *self.fps.lock().await = fps;
        *self.target.lock().await = Some(target.clone());
        *self.engine.lock().await = engine.clone();
        *self.start_time.lock().await = Some(tokio::time::Instant::now());
        self.is_streaming.store(true, Ordering::Relaxed);

        match engine {
            #[cfg(target_os = "windows")]
            CaptureEngine::Graphics => {
                self.start_graphics(app, target).await;
            }
            #[cfg(target_os = "windows")]
            CaptureEngine::Dxgi => {
                self.start_dxgi(app, fps, target).await;
            }
            CaptureEngine::Gdi => {
                self.start_gdi(app, fps, target).await;
            }
            #[cfg(not(target_os = "windows"))]
            _ => {
                self.start_gdi(app, fps, target).await;
            }
        }

        log::info!(
            "Stream started: fps={}, target={:?}, engine={:?}",
            fps,
            log_target,
            log_engine
        );
    }

    // ===== Windows-only: Graphics Capture =====

    #[cfg(target_os = "windows")]
    async fn start_graphics(&self, app: AppHandle, target: StreamTarget) {
        let sender = self.sender.clone();
        let start_time = self.start_time.clone();

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
                        },
                    ))
                })
                .await;

                let settings = match settings_result {
                    Ok(Ok(s)) => s,
                    Ok(Err(e)) => {
                        log::error!("Failed to create capture settings: {}", e);
                        *start_time.lock().await = None;
                        return;
                    }
                    Err(e) => {
                        log::error!("Capture settings thread panicked: {}", e);
                        *start_time.lock().await = None;
                        return;
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
                        },
                    ))
                })
                .await;

                let settings = match settings_result {
                    Ok(Ok(s)) => s,
                    Ok(Err(e)) => {
                        log::error!("Failed to create capture settings: {}", e);
                        *start_time.lock().await = None;
                        return;
                    }
                    Err(e) => {
                        log::error!("Capture settings thread panicked: {}", e);
                        *start_time.lock().await = None;
                        return;
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
                    }
                }
            }
        }
    }

    // ===== Windows-only: DXGI =====

    #[cfg(target_os = "windows")]
    async fn start_dxgi(&self, app: AppHandle, fps: u32, target: StreamTarget) {
        let is_streaming = self.is_streaming.clone();
        let sender = self.sender.clone();
        let target_clone = target.clone();
        let start_time = self.start_time.clone();

        let (monitor, crop) = match dxgi_capture_setup(&target_clone) {
            Ok(setup) => setup,
            Err(error) => {
                log::error!("Failed to prepare DXGI capture: {}", error);
                *start_time.lock().await = None;
                self.is_streaming.store(false, Ordering::Relaxed);
                return;
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
                        };
                        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(stream_frame.clone());
                        let _ = sender.send(stream_frame);
                        let _ = app.emit("stream-frame", ());
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
    }

    // ===== Cross-platform: GDI / xcap polling =====

    async fn start_gdi(&self, app: AppHandle, fps: u32, target: StreamTarget) {
        let is_streaming = self.is_streaming.clone();
        let sender = self.sender.clone();
        let target_clone = target.clone();
        let start_time = self.start_time.clone();

        let (width, height) = match &target_clone {
            StreamTarget::Screen { index } => match services::capture::get_screens() {
                Ok(screens) => {
                    if let Some(s) = screens.get(*index) {
                        (s.width, s.height)
                    } else {
                        (0, 0)
                    }
                }
                Err(_) => (0, 0),
            },
            StreamTarget::Window { hwnd, .. } => match services::capture::get_windows(None, None) {
                Ok(windows) => {
                    if let Some(w) = windows.iter().find(|w| w.hwnd == Some(*hwnd)) {
                        (w.width, w.height)
                    } else {
                        (0, 0)
                    }
                }
                Err(_) => (0, 0),
            },
        };

        let handle = tokio::spawn(async move {
            let interval_ms = 1000 / fps.max(1);

            loop {
                if !is_streaming.load(Ordering::Relaxed) {
                    break;
                }

                let target = target_clone.clone();
                let result = tokio::task::spawn_blocking(move || match &target {
                    StreamTarget::Screen { index } => {
                        services::capture::capture_screen_jpeg(*index)
                    }
                    StreamTarget::Window { hwnd, capture_area } => {
                        services::capture::capture_window_jpeg_by_hwnd(*hwnd, *capture_area)
                    }
                })
                .await
                .unwrap_or_else(|e| Err(crate::error::CaptureError::CaptureFailed(e.to_string())));

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
                        };
                        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(stream_frame.clone());
                        let _ = sender.send(stream_frame);
                        let _ = app.emit("stream-frame", ());
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
    }

    // ===== Shared: stop/status =====

    pub async fn stop(&self) {
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
        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        log::info!("Stream stopped");
    }

    pub fn stop_sync(&self) {
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

        *FRAME_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        log::info!("Stream stopped (sync)");
    }

    pub async fn status(&self) -> StreamStatus {
        let target = self.target.lock().await.clone();
        let fps = *self.fps.lock().await;
        let start_time = *self.start_time.lock().await;
        let engine = self.engine.lock().await.clone();

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

        let start_time_ms = start_time.map(|t| t.elapsed().as_millis() as u64);
        let engine_str = match engine {
            CaptureEngine::Gdi => "gdi".to_string(),
            #[cfg(target_os = "windows")]
            CaptureEngine::Dxgi => "dxgi".to_string(),
            CaptureEngine::Graphics => "graphics".to_string(),
        };

        StreamStatus {
            is_streaming: self.is_streaming.load(Ordering::Relaxed),
            fps,
            target_type,
            target_index,
            start_time: start_time_ms,
            engine: engine_str,
            capture_area,
        }
    }
}

impl Default for StreamService {
    fn default() -> Self {
        Self::new()
    }
}

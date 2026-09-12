use image::RgbaImage;
use xcap::Window;

use crate::error::CaptureError;
use crate::models::screen::ScreenInfo;
use crate::models::window::{WindowDetail, WindowInfo, WindowState};

use super::traits::PlatformProvider;

pub struct LinuxPlatform;

impl PlatformProvider for LinuxPlatform {
    fn list_all_windows(&self) -> Result<Vec<WindowInfo>, CaptureError> {
        let windows = Window::all().map_err(|e| CaptureError::CaptureFailed(e.to_string()))?;
        Ok(windows
            .iter()
            .enumerate()
            .map(|(i, w)| WindowInfo {
                index: i,
                title: w.title().unwrap_or_default().to_string(),
                app_name: w.app_name().unwrap_or_default().to_string(),
                width: w.width().unwrap_or(0),
                height: w.height().unwrap_or(0),
                hwnd: w.id().ok().map(|id| id as usize),
                desktop_id: None,
            })
            .collect())
    }

    fn list_windows_for_desktop(&self, _desktop_id: &str) -> Result<Vec<WindowInfo>, CaptureError> {
        // Linux has no virtual desktop concept in xcap — return all windows
        self.list_all_windows()
    }

    fn filter_windows(&self, windows: &mut Vec<WindowInfo>, filter: Option<WindowState>) {
        match filter {
            Some(WindowState::Foreground) => {
                windows.retain(|w| {
                    w.hwnd
                        .map(|hwnd| {
                            Window::all()
                                .ok()
                                .and_then(|all| {
                                    all.into_iter()
                                        .find(|win| win.id().ok() == Some(hwnd as u32))
                                })
                                .map(|w| !w.is_minimized().unwrap_or(true))
                                .unwrap_or(true)
                        })
                        .unwrap_or(true)
                });
            }
            Some(WindowState::Background) => {
                windows.retain(|w| {
                    w.hwnd
                        .map(|hwnd| {
                            Window::all()
                                .ok()
                                .and_then(|all| {
                                    all.into_iter()
                                        .find(|win| win.id().ok() == Some(hwnd as u32))
                                })
                                .map(|w| w.is_minimized().unwrap_or(false))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
                });
            }
            None => {}
        }
    }

    fn get_all_window_details(&self) -> Vec<WindowDetail> {
        Window::all()
            .ok()
            .map(|windows| {
                windows
                    .iter()
                    .map(|w| WindowDetail {
                        hwnd: w.id().ok().map(|id| id as usize).unwrap_or(0),
                        title: w.title().unwrap_or_default().to_string(),
                        class_name: String::new(),
                        process_name: w.app_name().unwrap_or_default().to_string(),
                        width: w.width().unwrap_or(0) as i32,
                        height: w.height().unwrap_or(0) as i32,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn capture_window(
        &self,
        hwnd: usize,
        _capture_area: crate::modules::pipeline::models::graph::CaptureArea,
    ) -> Result<RgbaImage, CaptureError> {
        let windows = Window::all().map_err(|e| CaptureError::CaptureFailed(e.to_string()))?;
        let window = windows
            .into_iter()
            .find(|w| w.id().ok() == Some(hwnd as u32))
            .ok_or(CaptureError::WindowNotFound)?;
        window
            .capture_image()
            .map_err(|e| CaptureError::CaptureFailed(e.to_string()))
    }

    fn list_screens(&self) -> Result<Vec<ScreenInfo>, CaptureError> {
        let monitors =
            xcap::Monitor::all().map_err(|e| CaptureError::CaptureFailed(e.to_string()))?;
        Ok(monitors
            .iter()
            .enumerate()
            .map(|(i, m)| ScreenInfo {
                index: i,
                name: m.name().unwrap_or_default().to_string(),
                width: m.width().unwrap_or(0),
                height: m.height().unwrap_or(0),
                is_primary: m.is_primary().unwrap_or(false),
            })
            .collect())
    }

    fn capture_screen(&self, index: usize) -> Result<RgbaImage, CaptureError> {
        let monitors =
            xcap::Monitor::all().map_err(|e| CaptureError::CaptureFailed(e.to_string()))?;
        let monitor = monitors
            .into_iter()
            .nth(index)
            .ok_or(CaptureError::ScreenNotFound)?;
        monitor
            .capture_image()
            .map_err(|e| CaptureError::CaptureFailed(e.to_string()))
    }
}

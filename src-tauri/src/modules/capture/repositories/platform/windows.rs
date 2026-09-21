#![allow(non_snake_case)]

use image::RgbaImage;
use std::ffi::c_void;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Xps::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_core::BOOL;

use crate::core::error::CaptureError;
use crate::modules::capture::models::screen::ScreenInfo;
use crate::modules::capture::models::target::CaptureArea;
use crate::modules::capture::models::window::{WindowDetail, WindowInfo, WindowState};
use crate::modules::capture::repositories::virtual_desktop::get_desktop_manager;
use crate::modules::capture::repositories::win32 as win32_helpers;

use super::traits::PlatformProvider;

pub struct WindowsPlatform;

impl WindowsPlatform {
    pub fn new() -> Self {
        Self
    }

    fn enum_all_hwnds(&self) -> Vec<isize> {
        unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let state = &mut *(lparam.0 as *mut (Vec<isize>, u32, u32, u32));
            if !IsWindowVisible(hwnd).as_bool() {
                state.2 += 1;
                return BOOL(1);
            }
            // Filter out tool windows (taskbar, overlays, system windows)
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
                state.3 += 1;
                return BOOL(1);
            }
            state.0.push(hwnd.0 as isize);
            state.1 += 1;
            BOOL(1)
        }
        let mut state: (Vec<isize>, u32, u32, u32) = (Vec::new(), 0, 0, 0);
        unsafe {
            let _ = EnumWindows(Some(callback), LPARAM(&mut state as *mut _ as isize));
        }
        log::info!(
            "[platform:windows] enum_all_hwnds: kept={}, hidden={}, tool_filtered={}",
            state.1,
            state.2,
            state.3
        );
        state.0
    }

    fn is_window_cloaked(hwnd: isize) -> bool {
        unsafe {
            let mut cloaked: i32 = 0;
            let hr = DwmGetWindowAttribute(
                HWND(hwnd as *mut c_void),
                DWMWA_CLOAKED,
                &mut cloaked as *mut i32 as *mut c_void,
                std::mem::size_of::<i32>() as u32,
            );
            hr.is_ok() && cloaked != 0
        }
    }

    fn hwnd_to_window_info(&self, hwnd: isize, index: usize) -> WindowInfo {
        let title = win32_helpers::get_window_title(hwnd);
        let app_name = self.get_window_app_name(hwnd);
        let rect = self.get_window_rect(hwnd);
        let width = rect.map(|r| r.2 as u32).unwrap_or(0);
        let height = rect.map(|r| r.3 as u32).unwrap_or(0);
        let desktop_id = get_desktop_manager()
            .get_window_desktop_id(hwnd as usize)
            .ok()
            .flatten();

        WindowInfo {
            index,
            title,
            app_name,
            width,
            height,
            hwnd: Some(hwnd as usize),
            desktop_id,
        }
    }

    fn get_window_app_name(&self, hwnd: isize) -> String {
        unsafe {
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(HWND(hwnd as *mut c_void), Some(&mut pid));
            if pid == 0 {
                return String::new();
            }
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);
            if handle.is_err() {
                return String::new();
            }
            let handle = handle.unwrap();
            let mut name_buf = [0u16; 260];
            let len = windows::Win32::System::ProcessStatus::GetModuleBaseNameW(
                handle,
                None,
                &mut name_buf,
            );
            let _ = CloseHandle(handle);
            if len > 0 {
                let name = String::from_utf16_lossy(&name_buf[..len as usize]);
                name.trim_end_matches(".exe").to_string()
            } else {
                String::new()
            }
        }
    }

    fn get_window_rect(&self, hwnd: isize) -> Option<(i32, i32, i32, i32)> {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(HWND(hwnd as *mut c_void), &mut rect).is_ok() {
                Some((
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                ))
            } else {
                None
            }
        }
    }

    fn hwnd_to_window_detail(&self, hwnd: isize) -> WindowDetail {
        let title = win32_helpers::get_window_title(hwnd);
        let class_name = win32_helpers::get_class_name(hwnd);
        let process_name = win32_helpers::get_window_process_name(hwnd);
        let rect = self.get_window_rect(hwnd);
        let width = rect.map(|r| r.2).unwrap_or(0);
        let height = rect.map(|r| r.3).unwrap_or(0);

        WindowDetail {
            hwnd: hwnd as usize,
            title,
            class_name,
            process_name,
            width,
            height,
        }
    }
}

impl PlatformProvider for WindowsPlatform {
    // ── Windows ──

    fn list_all_windows(&self) -> Result<Vec<WindowInfo>, CaptureError> {
        let vd_manager = get_desktop_manager();
        let hwnds = self.enum_all_hwnds();

        let windows: Vec<WindowInfo> = hwnds
            .iter()
            .enumerate()
            .filter_map(|(i, &hwnd)| {
                let is_on_current = match vd_manager.is_window_on_current_desktop(hwnd as usize) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("[platform:windows] is_window_on_current_desktop failed for hwnd {}: {:?}, including", hwnd, e);
                        true
                    }
                };
                if !is_on_current {
                    return None;
                }
                // Filter cloaked UWP windows (suspended apps on current desktop)
                if Self::is_window_cloaked(hwnd) {
                    return None;
                }
                Some(self.hwnd_to_window_info(hwnd, i))
            })
            .collect();

        log::info!(
            "[platform:windows] list_all_windows: {} windows",
            windows.len()
        );
        Ok(windows)
    }

    fn list_windows_for_desktop(&self, desktop_id: &str) -> Result<Vec<WindowInfo>, CaptureError> {
        let vd_manager = get_desktop_manager();
        let hwnds = self.enum_all_hwnds();

        let windows: Vec<WindowInfo> = hwnds
            .iter()
            .enumerate()
            .filter_map(|(i, &hwnd)| {
                let window_desktop_id = vd_manager
                    .get_window_desktop_id(hwnd as usize)
                    .ok()
                    .flatten();
                // Include window on COM error (fail-open), exclude if we can determine it's on a different desktop
                match &window_desktop_id {
                    Some(id) if id != desktop_id => None,
                    _ => Some(self.hwnd_to_window_info(hwnd, i)),
                }
            })
            .collect();

        log::info!(
            "[platform:windows] list_windows_for_desktop: {} windows for {}",
            windows.len(),
            desktop_id
        );
        Ok(windows)
    }

    fn filter_windows(&self, windows: &mut Vec<WindowInfo>, filter: Option<WindowState>) {
        match filter {
            Some(WindowState::Foreground) => {
                windows.retain(|w| {
                    w.hwnd
                        .map(|h| !win32_helpers::is_window_minimized(h as isize))
                        .unwrap_or(true)
                });
            }
            Some(WindowState::Background) => {
                windows.retain(|w| {
                    w.hwnd
                        .map(|h| win32_helpers::is_window_minimized(h as isize))
                        .unwrap_or(false)
                });
            }
            None => {}
        }
    }

    fn get_all_window_details(&self) -> Vec<WindowDetail> {
        let hwnds = self.enum_all_hwnds();
        hwnds
            .iter()
            .map(|&hwnd| self.hwnd_to_window_detail(hwnd))
            .collect()
    }

    fn capture_window(
        &self,
        hwnd: usize,
        capture_area: CaptureArea,
    ) -> Result<RgbaImage, CaptureError> {
        unsafe {
            let h = HWND(hwnd as isize as *mut c_void);
            if !IsWindow(Some(h)).as_bool() {
                return Err(CaptureError::WindowNotFound);
            }

            let mut rect = RECT::default();
            match capture_area {
                CaptureArea::Frame => GetWindowRect(h, &mut rect)
                    .map_err(|e| CaptureError::CaptureFailed(format!("GetWindowRect: {}", e)))?,
                CaptureArea::Client => GetClientRect(h, &mut rect)
                    .map_err(|e| CaptureError::CaptureFailed(format!("GetClientRect: {}", e)))?,
            }
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width <= 0 || height <= 0 {
                return Err(CaptureError::CaptureFailed(format!(
                    "Invalid window size: {}x{}",
                    width, height
                )));
            }

            let hdc = GetDC(Some(h));
            if hdc.is_invalid() {
                return Err(CaptureError::CaptureFailed("GetDC failed".into()));
            }

            let mem_dc = CreateCompatibleDC(Some(hdc));
            if mem_dc.is_invalid() {
                let _ = ReleaseDC(Some(h), hdc);
                return Err(CaptureError::CaptureFailed(
                    "CreateCompatibleDC failed".into(),
                ));
            }

            let bitmap = CreateCompatibleBitmap(hdc, width, height);
            if bitmap.is_invalid() {
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(Some(h), hdc);
                return Err(CaptureError::CaptureFailed(
                    "CreateCompatibleBitmap failed".into(),
                ));
            }

            let old_bitmap = SelectObject(mem_dc, bitmap.into());
            let print_flags = match capture_area {
                CaptureArea::Frame => PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT),
                CaptureArea::Client => PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT | PW_CLIENTONLY.0),
            };
            let render_ok = PrintWindow(h, mem_dc, print_flags);

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..std::mem::zeroed()
                },
                ..std::mem::zeroed()
            };

            let mut buf: Vec<u8> = vec![0u8; (width * height * 4) as usize];
            let result = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                Some(buf.as_mut_ptr() as *mut c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            let _ = SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(Some(h), hdc);

            if result == 0 && render_ok.as_bool() {
                return Err(CaptureError::CaptureFailed("GetDIBits failed".into()));
            }

            // BGRA → RGBA
            for chunk in buf.chunks_exact_mut(4) {
                chunk.swap(0, 2);
            }

            RgbaImage::from_raw(width as u32, height as u32, buf).ok_or_else(|| {
                CaptureError::CaptureFailed(format!("Failed to create image {}x{}", width, height))
            })
        }
    }

    // ── Screens ──

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

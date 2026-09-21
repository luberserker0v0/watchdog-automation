use crate::core::error::CaptureError;
use crate::modules::capture::models::target::{CaptureArea, PipelineTarget};
use crate::modules::capture::models::window::{WindowDetail, WindowInfo, WindowState};
use crate::modules::capture::repositories::platform::get_platform;

pub const WINDOW_SIZE_TOLERANCE: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WindowSizeMismatch {
    pub hwnd: usize,
    pub title: String,
    pub expected: [u32; 2],
    pub before: [u32; 2],
    pub actual: [u32; 2],
    pub resize_attempted: bool,
    pub diagnostic: String,
}

#[derive(Debug, Clone)]
pub struct WindowPlacementSnapshot {
    pub hwnd: usize,
    pub original_client_size: [u32; 2],
    pub flags: u32,
    pub show_cmd: u32,
    pub min_position: [i32; 2],
    pub max_position: [i32; 2],
    pub normal_rect: [i32; 4],
    pub was_maximized: bool,
    pub was_minimized: bool,
}

pub fn window_size_matches(actual: [u32; 2], expected: [u32; 2]) -> bool {
    actual[0].abs_diff(expected[0]) <= WINDOW_SIZE_TOLERANCE
        && actual[1].abs_diff(expected[1]) <= WINDOW_SIZE_TOLERANCE
}

pub fn corrected_outer_size(outer: [u32; 2], client: [u32; 2], expected: [u32; 2]) -> [u32; 2] {
    [
        outer[0]
            .saturating_add(expected[0])
            .saturating_sub(client[0]),
        outer[1]
            .saturating_add(expected[1])
            .saturating_sub(client[1]),
    ]
}

#[cfg(target_os = "windows")]
fn window_title(hwnd: windows::Win32::Foundation::HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    let mut buffer = [0u16; 512];
    let length = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

#[cfg(target_os = "windows")]
fn read_client_and_outer_size(
    hwnd: windows::Win32::Foundation::HWND,
) -> Result<([u32; 2], [u32; 2]), String> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};
    unsafe {
        let mut client = RECT::default();
        let mut outer = RECT::default();
        GetClientRect(hwnd, &mut client).map_err(|error| format!("GetClientRect: {error}"))?;
        GetWindowRect(hwnd, &mut outer).map_err(|error| format!("GetWindowRect: {error}"))?;
        Ok((
            [
                (client.right - client.left).max(0) as u32,
                (client.bottom - client.top).max(0) as u32,
            ],
            [
                (outer.right - outer.left).max(0) as u32,
                (outer.bottom - outer.top).max(0) as u32,
            ],
        ))
    }
}

#[cfg(target_os = "windows")]
pub fn get_window_client_size(hwnd_value: usize) -> Result<[u32; 2], String> {
    use core::ffi::c_void;
    use windows::Win32::Foundation::HWND;

    read_client_and_outer_size(HWND(hwnd_value as isize as *mut c_void)).map(|value| value.0)
}

#[cfg(not(target_os = "windows"))]
pub fn get_window_client_size(_hwnd: usize) -> Result<[u32; 2], String> {
    Err("Window client sizing is supported only on Windows".to_string())
}

fn validate_window_input_state(
    exists: bool,
    minimized: bool,
    foreground_hwnd: usize,
    target_hwnd: usize,
) -> Result<(), String> {
    if !exists {
        Err(format!(
            "Target window {target_hwnd} is no longer available"
        ))
    } else if minimized {
        Err(format!(
            "Target window {target_hwnd} is minimized and cannot receive global input"
        ))
    } else if foreground_hwnd != target_hwnd {
        Err(format!(
            "Target window {target_hwnd} is not foreground (foreground hwnd: {foreground_hwnd}); refusing global input"
        ))
    } else {
        Ok(())
    }
}

/// Restores and activates a window, then verifies that Windows actually granted
/// foreground ownership. Returns true when restoring/focusing changed state.
#[cfg(target_os = "windows")]
pub fn ensure_window_foreground(hwnd_value: usize) -> Result<bool, String> {
    use core::ffi::c_void;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsIconic, IsWindow, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    let hwnd = HWND(hwnd_value as isize as *mut c_void);
    let existed = unsafe { IsWindow(Some(hwnd)).as_bool() };
    if !existed {
        return validate_window_input_state(false, false, 0, hwnd_value).map(|_| false);
    }
    let was_minimized = unsafe { IsIconic(hwnd).as_bool() };
    let was_foreground = unsafe { GetForegroundWindow() == hwnd };
    if was_foreground && !was_minimized {
        return require_window_foreground(hwnd_value).map(|_| false);
    }
    if was_minimized {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
    }

    let deadline = Instant::now() + Duration::from_millis(1000);
    loop {
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        if require_window_foreground(hwnd_value).is_ok() {
            return Ok(was_minimized || !was_foreground);
        }
        if Instant::now() >= deadline {
            return require_window_foreground(hwnd_value).map(|_| was_minimized || !was_foreground);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_window_foreground(_hwnd: usize) -> Result<bool, String> {
    Err("Window foreground control is supported only on Windows".to_string())
}

/// Performs a non-mutating foreground check immediately before global input.
#[cfg(target_os = "windows")]
pub fn require_window_foreground(hwnd_value: usize) -> Result<(), String> {
    use core::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsIconic, IsWindow};

    let hwnd = HWND(hwnd_value as isize as *mut c_void);
    let exists = unsafe { IsWindow(Some(hwnd)).as_bool() };
    let minimized = exists && unsafe { IsIconic(hwnd).as_bool() };
    let foreground = unsafe { GetForegroundWindow() };
    validate_window_input_state(exists, minimized, foreground.0 as usize, hwnd_value)
}

#[cfg(not(target_os = "windows"))]
pub fn require_window_foreground(_hwnd: usize) -> Result<(), String> {
    Err("Window foreground verification is supported only on Windows".to_string())
}

#[cfg(target_os = "windows")]
pub fn prepare_window_size(
    hwnd_value: usize,
    expected: [u32; 2],
) -> Result<Option<WindowPlacementSnapshot>, WindowSizeMismatch> {
    use core::ffi::c_void;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, IsIconic, IsWindow, IsZoomed, SetWindowPos, ShowWindow, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOZORDER, SW_RESTORE, WINDOWPLACEMENT,
    };

    let hwnd = HWND(hwnd_value as isize as *mut c_void);
    let title = window_title(hwnd);
    let failure = |before, actual, diagnostic| WindowSizeMismatch {
        hwnd: hwnd_value,
        title: title.clone(),
        expected,
        before,
        actual,
        resize_attempted: true,
        diagnostic,
    };

    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err(failure(
                [0, 0],
                [0, 0],
                "Target window is no longer available".to_string(),
            ));
        }
    }
    let (before, _) =
        read_client_and_outer_size(hwnd).map_err(|error| failure([0, 0], [0, 0], error))?;
    if window_size_matches(before, expected) {
        return Ok(None);
    }

    let mut placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    unsafe {
        GetWindowPlacement(hwnd, &mut placement)
            .map_err(|error| failure(before, before, format!("GetWindowPlacement: {error}")))?;
    }
    let snapshot = WindowPlacementSnapshot {
        hwnd: hwnd_value,
        original_client_size: before,
        flags: placement.flags.0,
        show_cmd: placement.showCmd,
        min_position: [placement.ptMinPosition.x, placement.ptMinPosition.y],
        max_position: [placement.ptMaxPosition.x, placement.ptMaxPosition.y],
        normal_rect: [
            placement.rcNormalPosition.left,
            placement.rcNormalPosition.top,
            placement.rcNormalPosition.right,
            placement.rcNormalPosition.bottom,
        ],
        was_maximized: unsafe { IsZoomed(hwnd).as_bool() },
        was_minimized: unsafe { IsIconic(hwnd).as_bool() },
    };
    unsafe {
        if snapshot.was_maximized || snapshot.was_minimized {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
    }

    let started = Instant::now();
    let mut actual = before;
    let mut diagnostic =
        "The application constrained or rejected the requested client size".to_string();
    for _ in 0..3 {
        let (client, outer) = match read_client_and_outer_size(hwnd) {
            Ok(value) => value,
            Err(error) => {
                diagnostic = error;
                break;
            }
        };
        actual = client;
        if window_size_matches(actual, expected) {
            return Ok(Some(snapshot));
        }
        let desired_outer = corrected_outer_size(outer, client, expected);
        let resize = unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                desired_outer[0].min(i32::MAX as u32) as i32,
                desired_outer[1].min(i32::MAX as u32) as i32,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
            )
        };
        if let Err(error) = resize {
            diagnostic = format!("SetWindowPos: {error}");
            break;
        }
        let deadline = (started + Duration::from_millis(1500))
            .min(Instant::now() + Duration::from_millis(500));
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
            match read_client_and_outer_size(hwnd) {
                Ok((size, _)) => {
                    actual = size;
                    if window_size_matches(actual, expected) {
                        return Ok(Some(snapshot));
                    }
                }
                Err(error) => {
                    diagnostic = error;
                    break;
                }
            }
        }
    }

    let mut mismatch = failure(before, actual, diagnostic);
    if let Err(error) = restore_window_placement(&snapshot) {
        log::warn!("[window] failed to restore after resize mismatch: {error}");
        mismatch
            .diagnostic
            .push_str(&format!("; cleanup failed: {error}"));
    }
    Err(mismatch)
}

#[cfg(not(target_os = "windows"))]
pub fn prepare_window_size(
    hwnd: usize,
    expected: [u32; 2],
) -> Result<Option<WindowPlacementSnapshot>, WindowSizeMismatch> {
    Err(WindowSizeMismatch {
        hwnd,
        title: String::new(),
        expected,
        before: [0, 0],
        actual: [0, 0],
        resize_attempted: false,
        diagnostic: "Automatic window sizing is supported only on Windows".to_string(),
    })
}

#[cfg(target_os = "windows")]
pub fn restore_window_placement(snapshot: &WindowPlacementSnapshot) -> Result<(), String> {
    use core::ffi::c_void;
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindow, SetWindowPlacement, ShowWindow, SW_RESTORE, WINDOWPLACEMENT,
        WINDOWPLACEMENT_FLAGS,
    };
    let hwnd = HWND(snapshot.hwnd as isize as *mut c_void);
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            log::info!(
                "[window] skip placement restore; hwnd={} is closed",
                snapshot.hwnd
            );
            return Err(format!(
                "Window {} closed before placement restore",
                snapshot.hwnd
            ));
        }
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            flags: WINDOWPLACEMENT_FLAGS(snapshot.flags),
            showCmd: snapshot.show_cmd,
            ptMinPosition: POINT {
                x: snapshot.min_position[0],
                y: snapshot.min_position[1],
            },
            ptMaxPosition: POINT {
                x: snapshot.max_position[0],
                y: snapshot.max_position[1],
            },
            rcNormalPosition: RECT {
                left: snapshot.normal_rect[0],
                top: snapshot.normal_rect[1],
                right: snapshot.normal_rect[2],
                bottom: snapshot.normal_rect[3],
            },
        };
        SetWindowPlacement(hwnd, &placement)
            .map_err(|error| format!("Failed to restore window {}: {error}", snapshot.hwnd))?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn restore_window_placement(_snapshot: &WindowPlacementSnapshot) -> Result<(), String> {
    Err("Window placement restore is supported only on Windows".into())
}

fn match_regex_field(value: &str, pattern: &str) -> bool {
    regex_lite::Regex::new(pattern)
        .map(|re| re.is_match(value))
        .unwrap_or(false)
}

pub fn get_all_for_desktop(
    filter: Option<WindowState>,
    desktop_id: Option<String>,
) -> std::result::Result<Vec<WindowInfo>, CaptureError> {
    let platform = get_platform();
    let mut windows = match &desktop_id {
        Some(id) => platform.list_windows_for_desktop(id)?,
        None => platform.list_all_windows()?,
    };
    platform.filter_windows(&mut windows, filter);
    // Re-index
    windows
        .iter_mut()
        .enumerate()
        .for_each(|(i, w)| w.index = i);
    Ok(windows)
}

pub fn get_all_window_details() -> Vec<WindowDetail> {
    get_platform().get_all_window_details()
}

pub fn find_window_by_target(target: &PipelineTarget) -> std::result::Result<usize, String> {
    match target {
        PipelineTarget::Screen { index } => Ok(*index),
        PipelineTarget::Window {
            process,
            class,
            title,
            ..
        } => {
            if process.is_none() && class.is_none() && title.is_none() {
                return Err(
                    "Window target must specify at least one of: process, class, title".to_string(),
                );
            }

            let platform = get_platform();
            let details = platform.get_all_window_details();

            let mut best_match: Option<&WindowDetail> = None;

            for detail in &details {
                if detail.width <= 0 || detail.height <= 0 {
                    continue;
                }

                let mut matches = true;

                if let Some(process_pattern) = process {
                    if !match_regex_field(&detail.process_name, process_pattern) {
                        matches = false;
                    }
                }

                if let Some(class_pattern) = class {
                    if !match_regex_field(&detail.class_name, class_pattern) {
                        matches = false;
                    }
                }

                if let Some(title_pattern) = title {
                    if !match_regex_field(&detail.title, title_pattern) {
                        matches = false;
                    }
                }

                if matches {
                    if best_match.is_none()
                        || (best_match.as_ref().unwrap().title.is_empty()
                            && !detail.title.is_empty())
                    {
                        best_match = Some(detail);
                    }
                }
            }

            match best_match {
                Some(detail) => {
                    log::info!(
                        "[window] Resolved window: hwnd={}, title='{}', class='{}', process='{}'",
                        detail.hwnd,
                        detail.title,
                        detail.class_name,
                        detail.process_name
                    );
                    Ok(detail.hwnd)
                }
                None => {
                    let mut criteria = Vec::new();
                    if let Some(p) = process {
                        criteria.push(format!("process='{}'", p));
                    }
                    if let Some(c) = class {
                        criteria.push(format!("class='{}'", c));
                    }
                    if let Some(t) = title {
                        criteria.push(format!("title='{}'", t));
                    }
                    Err(format!("No window found matching: {}", criteria.join(", ")))
                }
            }
        }
    }
}

pub fn log_visible_windows() {
    let details = get_all_window_details();
    log::info!("[window] Visible windows ({}):", details.len());
    for d in &details {
        log::info!(
            "[window]   hwnd={}, title='{}', class='{}', process='{}', {}x{}",
            d.hwnd,
            d.title,
            d.class_name,
            d.process_name,
            d.width,
            d.height
        );
    }
}

pub fn encode_png(image: &image::RgbaImage) -> std::result::Result<Vec<u8>, CaptureError> {
    let mut buf = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| CaptureError::CaptureFailed(e.to_string()))?;
    Ok(buf.into_inner())
}

pub fn encode_jpeg(image: &image::RgbaImage) -> std::result::Result<Vec<u8>, CaptureError> {
    let jpeg = turbojpeg::compress_image(image, 85, turbojpeg::Subsamp::None)
        .map_err(|e| CaptureError::CaptureFailed(format!("JPEG encode error: {}", e)))?;
    Ok(jpeg.to_vec())
}

pub fn capture_by_hwnd(
    hwnd: usize,
    capture_area: CaptureArea,
) -> std::result::Result<Vec<u8>, CaptureError> {
    let image = get_platform().capture_window(hwnd, capture_area)?;
    encode_png(&image)
}

pub fn capture_as_jpeg_by_hwnd(
    hwnd: usize,
    capture_area: CaptureArea,
) -> std::result::Result<Vec<u8>, CaptureError> {
    let image = get_platform().capture_window(hwnd, capture_area)?;
    encode_jpeg(&image)
}

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn window_size_tolerance_is_two_pixels_per_dimension() {
        assert!(window_size_matches([1278, 722], [1280, 720]));
        assert!(!window_size_matches([1277, 720], [1280, 720]));
        assert!(!window_size_matches([1280, 723], [1280, 720]));
    }

    #[test]
    fn outer_size_correction_preserves_non_client_dimensions() {
        assert_eq!(
            corrected_outer_size([1296, 759], [1280, 720], [1024, 768]),
            [1040, 807]
        );
    }

    #[test]
    fn global_input_requires_the_exact_foreground_window() {
        assert!(validate_window_input_state(true, false, 42, 42).is_ok());
        assert!(validate_window_input_state(true, false, 7, 42)
            .unwrap_err()
            .contains("not foreground"));
    }

    #[test]
    fn global_input_rejects_closed_and_minimized_windows() {
        assert!(validate_window_input_state(false, false, 0, 42)
            .unwrap_err()
            .contains("no longer available"));
        assert!(validate_window_input_state(true, true, 42, 42)
            .unwrap_err()
            .contains("minimized"));
    }
}

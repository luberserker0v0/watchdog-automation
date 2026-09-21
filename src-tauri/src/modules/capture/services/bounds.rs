use crate::modules::capture::models::target::CaptureArea;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CaptureBounds {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

fn bounds_from_rect(rect: windows::Win32::Foundation::RECT) -> Option<CaptureBounds> {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    (width > 0 && height > 0).then_some(CaptureBounds {
        left: rect.left,
        top: rect.top,
        width: width as u32,
        height: height as u32,
    })
}

fn dimension_error(candidate: CaptureBounds, frame_width: u32, frame_height: u32) -> (u64, u64) {
    let aspect_error = (candidate.width as u64 * frame_height as u64)
        .abs_diff(candidate.height as u64 * frame_width as u64);
    let size_error = candidate.width.abs_diff(frame_width) as u64
        + candidate.height.abs_diff(frame_height) as u64;
    (aspect_error, size_error)
}

pub fn window_capture_bounds(
    hwnd: usize,
    capture_area: CaptureArea,
    frame_width: u32,
    frame_height: u32,
) -> Result<CaptureBounds, String> {
    use core::ffi::c_void;
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};

    unsafe {
        let hwnd = HWND(hwnd as isize as *mut c_void);
        if capture_area == CaptureArea::Client {
            let mut client_rect = RECT::default();
            GetClientRect(hwnd, &mut client_rect)
                .map_err(|e| format!("GetClientRect failed: {}", e))?;
            let mut origin = POINT::default();
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return Err("ClientToScreen failed".to_string());
            }
            return Ok(CaptureBounds {
                left: origin.x,
                top: origin.y,
                width: (client_rect.right - client_rect.left).max(0) as u32,
                height: (client_rect.bottom - client_rect.top).max(0) as u32,
            });
        }

        let mut window_rect = RECT::default();
        GetWindowRect(hwnd, &mut window_rect)
            .map_err(|e| format!("GetWindowRect failed: {}", e))?;
        let window_bounds = bounds_from_rect(window_rect)
            .ok_or_else(|| "GetWindowRect returned an empty rectangle".to_string())?;

        let mut dwm_rect = RECT::default();
        let dwm_bounds = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut dwm_rect as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
        .ok()
        .and_then(|_| bounds_from_rect(dwm_rect));

        Ok(match dwm_bounds {
            Some(dwm)
                if dimension_error(dwm, frame_width, frame_height)
                    < dimension_error(window_bounds, frame_width, frame_height) =>
            {
                dwm
            }
            _ => window_bounds,
        })
    }
}

pub fn screen_capture_bounds(index: usize) -> Result<CaptureBounds, String> {
    let monitor = xcap::Monitor::all()
        .map_err(|e| format!("Failed to enumerate monitors: {}", e))?
        .into_iter()
        .nth(index)
        .ok_or_else(|| format!("Screen index {} was not found", index))?;

    Ok(CaptureBounds {
        left: monitor
            .x()
            .map_err(|e| format!("Failed to read monitor x: {}", e))?,
        top: monitor
            .y()
            .map_err(|e| format!("Failed to read monitor y: {}", e))?,
        width: monitor
            .width()
            .map_err(|e| format!("Failed to read monitor width: {}", e))?,
        height: monitor
            .height()
            .map_err(|e| format!("Failed to read monitor height: {}", e))?,
    })
}

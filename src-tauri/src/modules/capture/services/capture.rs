use crate::core::error::CaptureError;
use crate::modules;

pub fn get_screens(
) -> Result<Vec<crate::modules::capture::models::screen::ScreenInfo>, CaptureError> {
    log::info!("Fetching screen list");
    modules::capture::repositories::screen::get_all()
}

pub fn get_windows(
    filter: Option<crate::modules::capture::models::window::WindowState>,
    desktop_id: Option<String>,
) -> Result<Vec<crate::modules::capture::models::window::WindowInfo>, CaptureError> {
    log::info!(
        "Fetching window list with filter: {:?}, desktop_id: {:?}",
        filter,
        desktop_id
    );
    modules::capture::repositories::window::get_all_for_desktop(filter, desktop_id)
}

pub fn capture_screen(index: usize) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing screen {}", index);
    modules::capture::repositories::screen::capture(index)
}

pub fn capture_screen_jpeg(index: usize) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing screen {} as JPEG", index);
    modules::capture::repositories::screen::capture_as_jpeg(index)
}

pub fn capture_window_by_hwnd(
    hwnd: usize,
    capture_area: crate::modules::capture::models::target::CaptureArea,
) -> Result<Vec<u8>, CaptureError> {
    // log::debug!("Capturing window by hwnd {}", hwnd);
    modules::capture::repositories::window::capture_by_hwnd(hwnd, capture_area)
}

pub fn capture_window_jpeg_by_hwnd(
    hwnd: usize,
    capture_area: crate::modules::capture::models::target::CaptureArea,
) -> Result<Vec<u8>, CaptureError> {
    // log::debug!("Capturing window {} as JPEG by hwnd", hwnd);
    modules::capture::repositories::window::capture_as_jpeg_by_hwnd(hwnd, capture_area)
}

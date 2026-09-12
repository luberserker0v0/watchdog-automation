use crate::error::CaptureError;
use crate::repositories;

pub fn get_screens() -> Result<Vec<crate::models::screen::ScreenInfo>, CaptureError> {
    log::info!("Fetching screen list");
    repositories::screen::get_all()
}

pub fn get_windows(
    filter: Option<crate::models::window::WindowState>,
    desktop_id: Option<String>,
) -> Result<Vec<crate::models::window::WindowInfo>, CaptureError> {
    log::info!(
        "Fetching window list with filter: {:?}, desktop_id: {:?}",
        filter,
        desktop_id
    );
    repositories::window::get_all_for_desktop(filter, desktop_id)
}

pub fn capture_screen(index: usize) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing screen {}", index);
    repositories::screen::capture(index)
}

pub fn capture_screen_jpeg(index: usize) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing screen {} as JPEG", index);
    repositories::screen::capture_as_jpeg(index)
}

pub fn capture_window_by_hwnd(
    hwnd: usize,
    capture_area: crate::modules::pipeline::models::graph::CaptureArea,
) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing window by hwnd {}", hwnd);
    repositories::window::capture_by_hwnd(hwnd, capture_area)
}

pub fn capture_window_jpeg_by_hwnd(
    hwnd: usize,
    capture_area: crate::modules::pipeline::models::graph::CaptureArea,
) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing window {} as JPEG by hwnd", hwnd);
    repositories::window::capture_as_jpeg_by_hwnd(hwnd, capture_area)
}

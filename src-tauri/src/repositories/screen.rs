use crate::error::CaptureError;
use crate::models::screen::ScreenInfo;
use crate::repositories::platform::get_platform;
use crate::repositories::window::{encode_jpeg, encode_png};

pub fn get_all() -> Result<Vec<ScreenInfo>, CaptureError> {
    log::info!("Fetching screen list");
    get_platform().list_screens()
}

pub fn capture(index: usize) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing screen {}", index);
    let image = get_platform().capture_screen(index)?;
    encode_png(&image)
}

pub fn capture_as_jpeg(index: usize) -> Result<Vec<u8>, CaptureError> {
    log::info!("Capturing screen {} as JPEG", index);
    let image = get_platform().capture_screen(index)?;
    encode_jpeg(&image)
}

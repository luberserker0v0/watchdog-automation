use image::RgbaImage;

use crate::error::CaptureError;
use crate::models::screen::ScreenInfo;
use crate::models::window::{WindowDetail, WindowInfo, WindowState};
use crate::modules::pipeline::models::graph::CaptureArea;

pub trait PlatformProvider: Send + Sync {
    // ── Window ──

    fn list_all_windows(&self) -> Result<Vec<WindowInfo>, CaptureError>;

    fn list_windows_for_desktop(&self, desktop_id: &str) -> Result<Vec<WindowInfo>, CaptureError>;

    fn filter_windows(&self, windows: &mut Vec<WindowInfo>, filter: Option<WindowState>);

    fn get_all_window_details(&self) -> Vec<WindowDetail>;

    fn capture_window(
        &self,
        hwnd: usize,
        capture_area: CaptureArea,
    ) -> Result<RgbaImage, CaptureError>;

    // ── Screen ──

    fn list_screens(&self) -> Result<Vec<ScreenInfo>, CaptureError>;

    fn capture_screen(&self, index: usize) -> Result<RgbaImage, CaptureError>;
}

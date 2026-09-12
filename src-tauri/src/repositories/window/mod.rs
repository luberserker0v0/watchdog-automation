use crate::error::CaptureError;
use crate::models::window::{WindowDetail, WindowInfo, WindowState};
use crate::modules::pipeline::models::graph::PipelineTarget;
use crate::repositories::platform::get_platform;

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
    capture_area: crate::modules::pipeline::models::graph::CaptureArea,
) -> std::result::Result<Vec<u8>, CaptureError> {
    let image = get_platform().capture_window(hwnd, capture_area)?;
    encode_png(&image)
}

pub fn capture_as_jpeg_by_hwnd(
    hwnd: usize,
    capture_area: crate::modules::pipeline::models::graph::CaptureArea,
) -> std::result::Result<Vec<u8>, CaptureError> {
    let image = get_platform().capture_window(hwnd, capture_area)?;
    encode_jpeg(&image)
}

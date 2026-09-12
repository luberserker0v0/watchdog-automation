use super::PluginError;
use crate::modules::pipeline::models::action::Target;
use crate::modules::pipeline::models::graph::CaptureArea;
use crate::modules::pipeline::models::recognizer::Region;
use std::collections::HashMap;

/// Resolves a node ROI against the captured frame.
///
/// `[0, 0, 0, 0]` is the persisted/default sentinel for "the whole frame".
/// Explicit ROIs are clipped to the frame so every recognizer uses the same,
/// safe crop semantics.
pub fn resolve_recognition_roi(
    roi: Option<&[i32; 4]>,
    frame_width: u32,
    frame_height: u32,
) -> Result<(u32, u32, u32, u32), PluginError> {
    if frame_width == 0 || frame_height == 0 {
        return Err(PluginError::RecognitionFailed(
            "Captured frame is empty".to_string(),
        ));
    }

    let Some([x, y, width, height]) = roi else {
        return Ok((0, 0, frame_width, frame_height));
    };
    if *x == 0 && *y == 0 && *width == 0 && *height == 0 {
        return Ok((0, 0, frame_width, frame_height));
    }
    if *x < 0 || *y < 0 || *width <= 0 || *height <= 0 {
        return Err(PluginError::RecognitionFailed(format!(
            "ROI must be 0,0,0,0 or positive x,y,width,height; received {x},{y},{width},{height}"
        )));
    }

    let x = *x as u32;
    let y = *y as u32;
    if x >= frame_width || y >= frame_height {
        return Err(PluginError::RecognitionFailed(format!(
            "ROI origin {x},{y} is outside frame {frame_width}x{frame_height}"
        )));
    }
    let width = (*width as u32).min(frame_width - x);
    let height = (*height as u32).min(frame_height - y);
    Ok((x, y, width, height))
}

pub fn resolve_recognition_roi_with_offset(
    roi: Option<&[i32; 4]>,
    roi_offset: Option<&[i32; 4]>,
    frame_width: u32,
    frame_height: u32,
) -> Result<[i32; 4], PluginError> {
    let (x, y, width, height) = resolve_recognition_roi(roi, frame_width, frame_height)?;
    let [offset_x, offset_y, offset_width, offset_height] =
        roi_offset.copied().unwrap_or([0, 0, 0, 0]);
    let adjusted = [
        x as i64 + offset_x as i64,
        y as i64 + offset_y as i64,
        width as i64 + offset_width as i64,
        height as i64 + offset_height as i64,
    ];
    if adjusted[0] < 0
        || adjusted[1] < 0
        || adjusted[2] <= 0
        || adjusted[3] <= 0
        || adjusted.iter().any(|value| *value > i32::MAX as i64)
    {
        return Err(PluginError::RecognitionFailed(format!(
            "ROI plus roi_offset is invalid: {},{},{},{}",
            adjusted[0], adjusted[1], adjusted[2], adjusted[3]
        )));
    }
    let adjusted = [
        adjusted[0] as i32,
        adjusted[1] as i32,
        adjusted[2] as i32,
        adjusted[3] as i32,
    ];
    let (x, y, width, height) =
        resolve_recognition_roi(Some(&adjusted), frame_width, frame_height)?;
    Ok([x as i32, y as i32, width as i32, height as i32])
}

/// Resolves an action target after applying its four-component offset.
/// Region targets (recognized box, node reference, or `[x,y,w,h]`) apply all
/// four values before taking the center. Point targets apply x/y only.
pub fn resolve_target_region_with_offset(
    target: &Target,
    target_offset: Option<&[i32; 4]>,
    box_region: Option<&Region>,
    prev_regions: &HashMap<String, Region>,
) -> Result<Region, PluginError> {
    let (mut region, is_point) = match target {
        Target::RecognizedBool(true) => (
            box_region.cloned().ok_or_else(|| {
                PluginError::ActionFailed("No recognition result for target".to_string())
            })?,
            false,
        ),
        Target::RecognizedBool(false) => {
            return Err(PluginError::ActionFailed(
                "Target is false, no recognition result".to_string(),
            ))
        }
        Target::Node(name) => (
            prev_regions.get(name).cloned().ok_or_else(|| {
                PluginError::ActionFailed(format!("Node '{}' has no recognition result", name))
            })?,
            false,
        ),
        Target::Coordinate([x, y]) => (
            Region {
                x: *x,
                y: *y,
                width: 0,
                height: 0,
            },
            true,
        ),
        Target::Region([x, y, width, height]) => {
            if *width < 0 || *height < 0 {
                return Err(PluginError::ActionFailed(
                    "Target region width and height cannot be negative".to_string(),
                ));
            }
            (
                Region {
                    x: *x,
                    y: *y,
                    width: *width as u32,
                    height: *height as u32,
                },
                false,
            )
        }
    };

    if let Some([offset_x, offset_y, offset_width, offset_height]) = target_offset {
        region.x = region.x.saturating_add(*offset_x);
        region.y = region.y.saturating_add(*offset_y);
        if !is_point {
            let width = region.width as i64 + *offset_width as i64;
            let height = region.height as i64 + *offset_height as i64;
            if width < 0 || height < 0 || width > u32::MAX as i64 || height > u32::MAX as i64 {
                return Err(PluginError::ActionFailed(
                    "target_offset produced an invalid target size".to_string(),
                ));
            }
            region.width = width as u32;
            region.height = height as u32;
        }
    }
    Ok(region)
}

pub fn resolve_target_with_offset(
    target: &Target,
    target_offset: Option<&[i32; 4]>,
    box_region: Option<&Region>,
    prev_regions: &HashMap<String, Region>,
) -> Result<(i32, i32), PluginError> {
    Ok(
        resolve_target_region_with_offset(target, target_offset, box_region, prev_regions)?
            .center(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CaptureBounds {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}

pub fn map_region(
    region: &Region,
    frame_width: u32,
    frame_height: u32,
    bounds: CaptureBounds,
) -> Region {
    let scale_x = bounds.width as f64 / frame_width.max(1) as f64;
    let scale_y = bounds.height as f64 / frame_height.max(1) as f64;

    Region {
        x: bounds.left + (region.x as f64 * scale_x).round() as i32,
        y: bounds.top + (region.y as f64 * scale_y).round() as i32,
        width: (region.width as f64 * scale_x).round().max(1.0) as u32,
        height: (region.height as f64 * scale_y).round().max(1.0) as u32,
    }
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

/// Maps a recognition region from captured window pixels to physical screen pixels.
///
/// Windows Graphics Capture uses the DWM extended frame (without the invisible
/// resize border), while PrintWindow/GDI uses GetWindowRect. Pick the rectangle
/// whose dimensions match the actual captured frame, then scale both the origin
/// and size. Scaling is required on mixed-DPI displays; translating only x/y makes
/// the eventual region centre incorrect.
pub fn window_region_to_screen(
    hwnd: usize,
    capture_area: CaptureArea,
    region: &Region,
    frame_width: u32,
    frame_height: u32,
) -> Result<Region, String> {
    let capture_bounds = window_capture_bounds(hwnd, capture_area, frame_width, frame_height)?;
    Ok(map_region(
        region,
        frame_width,
        frame_height,
        capture_bounds,
    ))
}

/// Maps a recognition region from a captured monitor to virtual-screen pixels.
/// Monitor origins may be negative when the display is left of or above primary.
pub fn screen_region_to_screen(
    index: usize,
    region: &Region,
    frame_width: u32,
    frame_height: u32,
) -> Result<Region, String> {
    let capture_bounds = screen_capture_bounds(index)?;
    Ok(map_region(
        region,
        frame_width,
        frame_height,
        capture_bounds,
    ))
}

#[cfg(test)]
mod coordinate_tests {
    use super::*;

    #[test]
    fn zero_roi_resolves_to_the_complete_frame() {
        assert_eq!(
            resolve_recognition_roi(Some(&[0, 0, 0, 0]), 640, 480).unwrap(),
            (0, 0, 640, 480)
        );
        assert_eq!(
            resolve_recognition_roi(None, 640, 480).unwrap(),
            (0, 0, 640, 480)
        );
    }

    #[test]
    fn explicit_roi_is_clipped_and_invalid_zero_size_is_rejected() {
        assert_eq!(
            resolve_recognition_roi(Some(&[600, 450, 100, 100]), 640, 480).unwrap(),
            (600, 450, 40, 30)
        );
        assert!(resolve_recognition_roi(Some(&[10, 10, 0, 20]), 640, 480).is_err());
    }

    #[test]
    fn roi_offset_changes_the_effective_recognition_scope() {
        assert_eq!(
            resolve_recognition_roi_with_offset(
                Some(&[10, 20, 100, 80]),
                Some(&[5, -5, 20, -10]),
                640,
                480,
            )
            .unwrap(),
            [15, 15, 120, 70]
        );
        assert_eq!(
            resolve_recognition_roi_with_offset(
                Some(&[0, 0, 0, 0]),
                Some(&[10, 20, -30, -40]),
                640,
                480,
            )
            .unwrap(),
            [10, 20, 610, 440]
        );
    }

    #[test]
    fn target_offset_adjusts_regions_before_resolving_the_center() {
        let box_region = Region {
            x: 100,
            y: 200,
            width: 80,
            height: 40,
        };
        let regions = HashMap::from([("previous".to_string(), box_region.clone())]);
        assert_eq!(
            resolve_target_with_offset(
                &Target::RecognizedBool(true),
                Some(&[10, -20, 20, 10]),
                Some(&box_region),
                &regions,
            )
            .unwrap(),
            (160, 205)
        );
        assert_eq!(
            resolve_target_with_offset(
                &Target::Node("previous".to_string()),
                Some(&[0, 10, 0, 0]),
                None,
                &regions,
            )
            .unwrap(),
            (140, 230)
        );
        assert_eq!(
            resolve_target_with_offset(
                &Target::Coordinate([30, 40]),
                Some(&[5, 6, 99, 99]),
                None,
                &regions,
            )
            .unwrap(),
            (35, 46)
        );
    }

    #[test]
    fn maps_translation_and_region_size() {
        let region = Region {
            x: 100,
            y: 50,
            width: 40,
            height: 20,
        };
        let mapped = map_region(
            &region,
            500,
            600,
            CaptureBounds {
                left: -200,
                top: 80,
                width: 500,
                height: 600,
            },
        );
        assert_eq!(
            mapped,
            Region {
                x: -100,
                y: 130,
                width: 40,
                height: 20
            }
        );
        assert_eq!(mapped.center(), (-80, 140));
    }

    #[test]
    fn scales_position_and_size_before_resolving_center() {
        let region = Region {
            x: 100,
            y: 50,
            width: 40,
            height: 20,
        };
        let mapped = map_region(
            &region,
            500,
            300,
            CaptureBounds {
                left: 10,
                top: 20,
                width: 1000,
                height: 600,
            },
        );
        assert_eq!(
            mapped,
            Region {
                x: 210,
                y: 120,
                width: 80,
                height: 40
            }
        );
        assert_eq!(mapped.center(), (250, 140));
    }
}

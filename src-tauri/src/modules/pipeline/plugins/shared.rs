use super::PluginError;
use crate::modules::pipeline::models::action::{Action, Target};
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

    resolve_recognition_roi_with_offset(roi, None, frame_width, frame_height)
        .map(|[x, y, width, height]| (x as u32, y as u32, width as u32, height as u32))
}

pub fn resolve_recognition_roi_with_offset(
    roi: Option<&[i32; 4]>,
    roi_offset: Option<&[i32; 4]>,
    frame_width: u32,
    frame_height: u32,
) -> Result<[i32; 4], PluginError> {
    if frame_width == 0 || frame_height == 0 {
        return Err(PluginError::RecognitionFailed(
            "Captured frame is empty".to_string(),
        ));
    }

    let requested = match roi {
        None | Some([0, 0, 0, 0]) => [0_i64, 0, frame_width as i64, frame_height as i64],
        Some([x, y, width, height]) => {
            if *width <= 0 || *height <= 0 {
                return Err(PluginError::RecognitionFailed(format!(
                    "ROI width and height must be positive; received {x},{y},{width},{height}"
                )));
            }
            [*x as i64, *y as i64, *width as i64, *height as i64]
        }
    };
    let [offset_x, offset_y, offset_width, offset_height] =
        roi_offset.copied().unwrap_or([0, 0, 0, 0]);
    let adjusted = [
        requested[0] + offset_x as i64,
        requested[1] + offset_y as i64,
        requested[2] + offset_width as i64,
        requested[3] + offset_height as i64,
    ];
    if adjusted[2] <= 0 || adjusted[3] <= 0 {
        return Err(PluginError::RecognitionFailed(format!(
            "ROI plus roi_offset is invalid: {},{},{},{}",
            adjusted[0], adjusted[1], adjusted[2], adjusted[3]
        )));
    }
    let right = adjusted[0]
        .checked_add(adjusted[2])
        .ok_or_else(|| PluginError::RecognitionFailed("ROI right edge overflowed".to_string()))?;
    let bottom = adjusted[1]
        .checked_add(adjusted[3])
        .ok_or_else(|| PluginError::RecognitionFailed("ROI bottom edge overflowed".to_string()))?;
    let left = adjusted[0].max(0);
    let top = adjusted[1].max(0);
    let right = right.min(frame_width as i64);
    let bottom = bottom.min(frame_height as i64);
    if left >= right || top >= bottom {
        return Err(PluginError::RecognitionFailed(format!(
            "ROI {},{},{},{} does not overlap frame {}x{}",
            adjusted[0], adjusted[1], adjusted[2], adjusted[3], frame_width, frame_height
        )));
    }
    Ok([
        left as i32,
        top as i32,
        (right - left) as i32,
        (bottom - top) as i32,
    ])
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
        region.x = region.x.checked_add(*offset_x).ok_or_else(|| {
            PluginError::ActionFailed(
                "target_offset overflowed the target x coordinate".to_string(),
            )
        })?;
        region.y = region.y.checked_add(*offset_y).ok_or_else(|| {
            PluginError::ActionFailed(
                "target_offset overflowed the target y coordinate".to_string(),
            )
        })?;
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

/// Intersects recognizer output with the encoded frame. Recognition geometry is
/// safe to clip; action geometry deliberately uses strict validation below.
pub fn clip_region_to_frame(
    region: &Region,
    frame_width: u32,
    frame_height: u32,
) -> Option<Region> {
    if frame_width == 0 || frame_height == 0 || region.width == 0 || region.height == 0 {
        return None;
    }
    let left = (region.x as i64).max(0);
    let top = (region.y as i64).max(0);
    let right = (region.x as i64 + region.width as i64).min(frame_width as i64);
    let bottom = (region.y as i64 + region.height as i64).min(frame_height as i64);
    (left < right && top < bottom).then_some(Region {
        x: left as i32,
        y: top as i32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

fn region_edges(region: &Region) -> Result<(i64, i64, i64, i64), PluginError> {
    let left = region.x as i64;
    let top = region.y as i64;
    let right = left.checked_add(region.width as i64).ok_or_else(|| {
        PluginError::ActionFailed("Target region right edge overflowed".to_string())
    })?;
    let bottom = top.checked_add(region.height as i64).ok_or_else(|| {
        PluginError::ActionFailed("Target region bottom edge overflowed".to_string())
    })?;
    Ok((left, top, right, bottom))
}

/// Requires an action target to remain completely inside a capture coordinate space.
/// Zero-sized regions are points and use half-open bounds (`0 <= x < width`).
pub fn validate_local_action_region(
    region: &Region,
    frame_width: u32,
    frame_height: u32,
) -> Result<(), PluginError> {
    if frame_width == 0 || frame_height == 0 {
        return Err(PluginError::ActionFailed(
            "Captured frame is empty".to_string(),
        ));
    }
    let (left, top, right, bottom) = region_edges(region)?;
    let point = region.width == 0 && region.height == 0;
    let inside = if point {
        left >= 0 && top >= 0 && left < frame_width as i64 && top < frame_height as i64
    } else {
        region.width > 0
            && region.height > 0
            && left >= 0
            && top >= 0
            && right <= frame_width as i64
            && bottom <= frame_height as i64
    };
    if inside {
        Ok(())
    } else {
        Err(PluginError::ActionFailed(format!(
            "Target {},{},{},{} is outside capture frame {}x{} (valid x: 0..{}, y: 0..{})",
            region.x,
            region.y,
            region.width,
            region.height,
            frame_width,
            frame_height,
            frame_width - 1,
            frame_height - 1
        )))
    }
}

pub fn action_anchor(region: &Region) -> Result<Region, PluginError> {
    let x = i32::try_from(region.x as i64 + region.width as i64 / 2).map_err(|_| {
        PluginError::ActionFailed("Resolved target center x overflowed i32".to_string())
    })?;
    let y = i32::try_from(region.y as i64 + region.height as i64 / 2).map_err(|_| {
        PluginError::ActionFailed("Resolved target center y overflowed i32".to_string())
    })?;
    Ok(Region {
        x,
        y,
        width: 0,
        height: 0,
    })
}

pub fn validate_screen_action_region(
    region: &Region,
    bounds: CaptureBounds,
) -> Result<(), PluginError> {
    let (left, top, right, bottom) = region_edges(region)?;
    let bound_right = bounds.left as i64 + bounds.width as i64;
    let bound_bottom = bounds.top as i64 + bounds.height as i64;
    let point = region.width == 0 && region.height == 0;
    let inside = if point {
        left >= bounds.left as i64
            && top >= bounds.top as i64
            && left < bound_right
            && top < bound_bottom
    } else {
        region.width > 0
            && region.height > 0
            && left >= bounds.left as i64
            && top >= bounds.top as i64
            && right <= bound_right
            && bottom <= bound_bottom
    };
    if inside {
        Ok(())
    } else {
        Err(PluginError::ActionFailed(format!(
            "Mapped target {},{},{},{} is outside screen bounds {},{},{},{}",
            region.x,
            region.y,
            region.width,
            region.height,
            bounds.left,
            bounds.top,
            bounds.width,
            bounds.height
        )))
    }
}

fn map_axis(
    value: i64,
    source_size: u32,
    target_origin: i32,
    target_size: u32,
) -> Result<i32, PluginError> {
    if source_size == 0 || target_size == 0 {
        return Err(PluginError::ActionFailed(
            "Cannot map through empty bounds".to_string(),
        ));
    }
    let numerator = value as i128 * target_size as i128;
    let mapped =
        target_origin as i128 + (numerator + source_size as i128 / 2) / source_size as i128;
    i32::try_from(mapped)
        .map_err(|_| PluginError::ActionFailed("Mapped coordinate overflowed i32".to_string()))
}

pub fn map_region_checked(
    region: &Region,
    frame_width: u32,
    frame_height: u32,
    bounds: CaptureBounds,
) -> Result<Region, PluginError> {
    validate_local_action_region(region, frame_width, frame_height)?;
    let (left, top, right, bottom) = region_edges(region)?;
    let mapped_left = map_axis(left, frame_width, bounds.left, bounds.width)?;
    let mapped_top = map_axis(top, frame_height, bounds.top, bounds.height)?;
    let mapped_right = map_axis(right, frame_width, bounds.left, bounds.width)?;
    let mapped_bottom = map_axis(bottom, frame_height, bounds.top, bounds.height)?;
    if (region.width > 0 && mapped_right <= mapped_left)
        || (region.height > 0 && mapped_bottom <= mapped_top)
    {
        return Err(PluginError::ActionFailed(
            "Target region collapsed while mapping to screen pixels".to_string(),
        ));
    }
    let mapped = Region {
        x: mapped_left,
        y: mapped_top,
        width: mapped_right.abs_diff(mapped_left),
        height: mapped_bottom.abs_diff(mapped_top),
    };
    validate_screen_action_region(&mapped, bounds)?;
    Ok(mapped)
}

fn map_action_target_checked(
    target: &Target,
    offset: Option<&[i32; 4]>,
    box_region: Option<&Region>,
    prev_regions: &HashMap<String, Region>,
    frame_width: u32,
    frame_height: u32,
    bounds: CaptureBounds,
) -> Result<Target, PluginError> {
    let local = resolve_target_region_with_offset(target, offset, box_region, prev_regions)?;
    // Built-in pointer actions execute at the resolved region's centre. The
    // region itself may extend beyond an image edge after an offset, but that is
    // harmless as long as the actual input anchor remains inside the frame.
    // Validate and map the primitive we really send to the OS, not its visual
    // source rectangle.
    let anchor = action_anchor(&local)?;
    let mapped = map_region_checked(&anchor, frame_width, frame_height, bounds)?;
    Ok(Target::Coordinate([mapped.x, mapped.y]))
}

/// Resolves all built-in positional action targets in capture-local pixels,
/// validates them, and converts them to physical screen pixels.
pub fn map_action_checked(
    action: &Action,
    box_region: Option<&Region>,
    prev_regions: &HashMap<String, Region>,
    frame_width: u32,
    frame_height: u32,
    bounds: CaptureBounds,
) -> Result<Action, PluginError> {
    Ok(match action {
        Action::Click {
            target,
            target_offset,
        } => Action::Click {
            target: map_action_target_checked(
                target,
                target_offset.as_ref(),
                box_region,
                prev_regions,
                frame_width,
                frame_height,
                bounds,
            )?,
            target_offset: None,
        },
        Action::LongPress {
            target,
            target_offset,
            duration,
        } => Action::LongPress {
            target: map_action_target_checked(
                target,
                target_offset.as_ref(),
                box_region,
                prev_regions,
                frame_width,
                frame_height,
                bounds,
            )?,
            target_offset: None,
            duration: *duration,
        },
        Action::Swipe {
            begin,
            begin_offset,
            end,
            end_offset,
            duration,
        } => Action::Swipe {
            begin: map_action_target_checked(
                begin,
                begin_offset.as_ref(),
                box_region,
                prev_regions,
                frame_width,
                frame_height,
                bounds,
            )?,
            begin_offset: None,
            end: map_action_target_checked(
                end,
                end_offset.as_ref(),
                box_region,
                prev_regions,
                frame_width,
                frame_height,
                bounds,
            )?,
            end_offset: None,
            duration: *duration,
        },
        Action::Scroll {
            target,
            target_offset,
            dx,
            dy,
        } => Action::Scroll {
            target: map_action_target_checked(
                target,
                target_offset.as_ref(),
                box_region,
                prev_regions,
                frame_width,
                frame_height,
                bounds,
            )?,
            target_offset: None,
            dx: *dx,
            dy: *dy,
        },
        other => other.clone(),
    })
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
    fn recognition_roi_clips_every_edge_and_rejects_no_overlap() {
        assert_eq!(
            resolve_recognition_roi(Some(&[-20, -10, 50, 40]), 640, 480).unwrap(),
            (0, 0, 30, 30)
        );
        assert!(resolve_recognition_roi(Some(&[-100, 10, 20, 20]), 640, 480).is_err());
        assert!(resolve_recognition_roi(Some(&[640, 10, 20, 20]), 640, 480).is_err());
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
    fn checked_action_mapping_rejects_points_and_regions_outside_frame() {
        let bounds = CaptureBounds {
            left: -200,
            top: 80,
            width: 1280,
            height: 720,
        };
        assert!(map_region_checked(
            &Region {
                x: 1280,
                y: 10,
                width: 0,
                height: 0
            },
            1280,
            720,
            bounds,
        )
        .is_err());
        assert!(map_region_checked(
            &Region {
                x: 1200,
                y: 650,
                width: 100,
                height: 80
            },
            1280,
            720,
            bounds,
        )
        .is_err());
    }

    #[test]
    fn checked_action_mapping_supports_negative_monitor_origins() {
        let mapped = map_region_checked(
            &Region {
                x: 640,
                y: 360,
                width: 0,
                height: 0,
            },
            1280,
            720,
            CaptureBounds {
                left: -1280,
                top: -100,
                width: 1280,
                height: 720,
            },
        )
        .unwrap();
        assert_eq!(
            mapped,
            Region {
                x: -640,
                y: 260,
                width: 0,
                height: 0
            }
        );
    }

    #[test]
    fn swipe_allows_shifted_source_region_when_actual_anchor_is_inside() {
        let action = Action::Swipe {
            begin: Target::RecognizedBool(true),
            begin_offset: Some([-100, 0, 0, 0]),
            end: Target::RecognizedBool(true),
            end_offset: Some([0, 100, 0, 0]),
            duration: 200,
        };
        let mapped = map_action_checked(
            &action,
            Some(&Region {
                x: 64,
                y: 10,
                width: 453,
                height: 358,
            }),
            &HashMap::new(),
            500,
            600,
            CaptureBounds {
                left: 0,
                top: 0,
                width: 500,
                height: 600,
            },
        )
        .unwrap();
        match mapped {
            Action::Swipe { begin, end, .. } => {
                assert!(matches!(begin, Target::Coordinate([190, 189])));
                assert!(matches!(end, Target::Coordinate([290, 289])));
            }
            _ => panic!("expected swipe"),
        }
    }

    #[test]
    fn action_rejects_shifted_region_when_its_anchor_is_outside() {
        let action = Action::Click {
            target: Target::Region([-500, 10, 453, 358]),
            target_offset: None,
        };
        assert!(map_action_checked(
            &action,
            None,
            &HashMap::new(),
            500,
            600,
            CaptureBounds {
                left: 0,
                top: 0,
                width: 500,
                height: 600,
            },
        )
        .is_err());
    }

    #[test]
    fn target_offset_overflow_is_an_error_instead_of_saturating() {
        let result = resolve_target_region_with_offset(
            &Target::Coordinate([i32::MAX, 10]),
            Some(&[1, 0, 0, 0]),
            None,
            &HashMap::new(),
        );
        assert!(result.is_err());
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

use std::collections::HashMap;

use crate::modules::capture::services::bounds::{
    screen_capture_bounds, window_capture_bounds, CaptureBounds,
};
use crate::modules::pipeline::authoring::models::action::{Action, Target};
use crate::modules::pipeline::authoring::models::graph::CaptureArea;
use crate::modules::pipeline::authoring::models::recognizer::Region;

use super::{resolve_target_region_with_offset, PluginError};
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

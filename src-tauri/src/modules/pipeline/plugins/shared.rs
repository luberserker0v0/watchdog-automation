mod coordinates;

pub use coordinates::*;

use super::PluginError;
pub use crate::modules::capture::services::bounds::{
    screen_capture_bounds, window_capture_bounds, CaptureBounds,
};
use crate::modules::capture::services::stream::StreamTarget;
#[cfg(test)]
use crate::modules::pipeline::authoring::models::action::Action;
use crate::modules::pipeline::authoring::models::action::Target;
#[cfg(test)]
use crate::modules::pipeline::authoring::models::graph::CaptureArea;
use crate::modules::pipeline::authoring::models::recognizer::{RecognitionRoi, Region};
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

/// Convert a prior node's screen-space box into the current capture frame.
pub fn resolve_node_recognition_roi(
    roi: Option<&RecognitionRoi>,
    roi_offset: Option<&[i32; 4]>,
    frame_width: u32,
    frame_height: u32,
    bounds: Option<CaptureBounds>,
    current_target: &StreamTarget,
    prior_regions: &HashMap<String, Region>,
    prior_targets: &HashMap<String, StreamTarget>,
) -> Result<[i32; 4], PluginError> {
    let fixed = match roi {
        None => None,
        Some(RecognitionRoi::Fixed(rect)) => Some(*rect),
        Some(RecognitionRoi::Node(id)) => {
            if id.trim().is_empty() {
                return Err(PluginError::RecognitionFailed(
                    "ROI node reference is empty".into(),
                ));
            }
            let region = prior_regions.get(id).ok_or_else(|| {
                PluginError::RecognitionFailed(format!(
                    "ROI node '{id}' has no successful recognition box in this run"
                ))
            })?;
            let prior_target = prior_targets.get(id).ok_or_else(|| {
                PluginError::RecognitionFailed(format!(
                    "ROI node '{id}' has no capture target metadata"
                ))
            })?;
            let same_identity = match (prior_target, current_target) {
                (StreamTarget::Window { hwnd: a, .. }, StreamTarget::Window { hwnd: b, .. }) => {
                    a == b
                }
                (StreamTarget::Screen { index: a }, StreamTarget::Screen { index: b }) => a == b,
                _ => false,
            };
            if !same_identity {
                return Err(PluginError::RecognitionFailed(format!(
                    "ROI node '{id}' belongs to another capture target"
                )));
            }
            let bounds = bounds.ok_or_else(|| {
                PluginError::RecognitionFailed(
                    "Cannot resolve ROI node without current frame bounds".into(),
                )
            })?;
            if bounds.width == 0 || bounds.height == 0 || frame_width == 0 || frame_height == 0 {
                return Err(PluginError::RecognitionFailed(
                    "Cannot resolve ROI against an empty frame".into(),
                ));
            }
            let left = ((region.x as i64 - bounds.left as i64) as f64 * frame_width as f64
                / bounds.width as f64)
                .floor() as i64;
            let top = ((region.y as i64 - bounds.top as i64) as f64 * frame_height as f64
                / bounds.height as f64)
                .floor() as i64;
            let right = ((region.x as i64 + region.width as i64 - bounds.left as i64) as f64
                * frame_width as f64
                / bounds.width as f64)
                .ceil() as i64;
            let bottom = ((region.y as i64 + region.height as i64 - bounds.top as i64) as f64
                * frame_height as f64
                / bounds.height as f64)
                .ceil() as i64;
            let values = [left, top, right - left, bottom - top];
            let rect: [i32; 4] = values.map(|value| {
                i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
            });
            Some(rect)
        }
    };
    resolve_recognition_roi_with_offset(fixed.as_ref(), roi_offset, frame_width, frame_height)
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

/// Intersects recognizer output with the encoded frame. Recognition geometry is
/// safe to clip; action geometry deliberately uses strict validation below.
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
    fn node_roi_maps_screen_box_into_client_frame_then_applies_offset() {
        let id = "FindPanel".to_string();
        let prior = HashMap::from([(
            id.clone(),
            Region {
                x: -90,
                y: 70,
                width: 80,
                height: 40,
            },
        )]);
        let frame_target = StreamTarget::Window {
            hwnd: 42,
            capture_area: CaptureArea::Frame,
        };
        let client_target = StreamTarget::Window {
            hwnd: 42,
            capture_area: CaptureArea::Client,
        };
        let targets = HashMap::from([(id.clone(), frame_target)]);
        let bounds = CaptureBounds {
            left: -100,
            top: 50,
            width: 400,
            height: 200,
        };
        assert_eq!(
            resolve_node_recognition_roi(
                Some(&RecognitionRoi::Node(id)),
                Some(&[2, 3, 4, -2]),
                800,
                400,
                Some(bounds),
                &client_target,
                &prior,
                &targets,
            )
            .unwrap(),
            [22, 43, 164, 78]
        );
    }

    #[test]
    fn node_roi_rejects_missing_and_incompatible_targets() {
        let id = "FindPanel".to_string();
        let roi = RecognitionRoi::Node(id.clone());
        let prior = HashMap::from([(
            id.clone(),
            Region {
                x: 10,
                y: 10,
                width: 20,
                height: 20,
            },
        )]);
        let target = StreamTarget::Screen { index: 1 };
        let bounds = Some(CaptureBounds {
            left: 0,
            top: 0,
            width: 100,
            height: 100,
        });
        assert!(resolve_node_recognition_roi(
            Some(&roi),
            None,
            100,
            100,
            bounds,
            &target,
            &HashMap::new(),
            &HashMap::new()
        )
        .is_err());
        let targets = HashMap::from([(id, StreamTarget::Screen { index: 0 })]);
        assert!(resolve_node_recognition_roi(
            Some(&roi),
            None,
            100,
            100,
            bounds,
            &target,
            &prior,
            &targets
        )
        .is_err());
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

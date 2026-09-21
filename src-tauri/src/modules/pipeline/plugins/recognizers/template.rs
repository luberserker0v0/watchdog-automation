use crate::core::resource_resolver;
use crate::modules::pipeline::authoring::models::constants::MAX_RGB_DIFF_SQ;
use crate::modules::pipeline::authoring::models::recognizer::{
    MatchCandidate, MatchResult, Region,
};
use crate::modules::pipeline::plugins::shared::resolve_recognition_roi;
use crate::modules::pipeline::plugins::{PluginError, RecognizerPlugin};
use image::DynamicImage;
use std::collections::HashMap;

pub struct TemplateRecognizer;

const TEMPLATE_NMS_IOU: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
struct RawDetection {
    avg_diff: f64,
    x: u32,
    y: u32,
    scan_index: usize,
}

fn intersection_over_union(a: RawDetection, b: RawDetection, width: u32, height: u32) -> f64 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = a.x.saturating_add(width).min(b.x.saturating_add(width));
    let bottom = a.y.saturating_add(height).min(b.y.saturating_add(height));
    if right <= left || bottom <= top {
        return 0.0;
    }
    let intersection = (right - left) as u64 * (bottom - top) as u64;
    let area = width as u64 * height as u64;
    intersection as f64 / (area * 2 - intersection) as f64
}

fn suppress_overlaps(
    sorted: Vec<RawDetection>,
    width: u32,
    height: u32,
    iou_threshold: f64,
) -> Vec<RawDetection> {
    let cell_width = width.max(1);
    let cell_height = height.max(1);
    let mut kept = Vec::<RawDetection>::new();
    let mut grid = HashMap::<(i64, i64), Vec<usize>>::new();

    for candidate in sorted {
        let cell_x = (candidate.x / cell_width) as i64;
        let cell_y = (candidate.y / cell_height) as i64;
        let overlaps = (-1..=1).any(|dy| {
            (-1..=1).any(|dx| {
                grid.get(&(cell_x + dx, cell_y + dy))
                    .is_some_and(|indices| {
                        indices.iter().any(|index| {
                            intersection_over_union(candidate, kept[*index], width, height)
                                >= iou_threshold
                        })
                    })
            })
        });
        if !overlaps {
            let index = kept.len();
            kept.push(candidate);
            grid.entry((cell_x, cell_y)).or_default().push(index);
        }
    }
    kept
}

impl RecognizerPlugin for TemplateRecognizer {
    fn name(&self) -> &str {
        "TemplateMatch"
    }

    fn evaluate(
        &self,
        screenshot: &DynamicImage,
        recognition: &Recognition,
        roi: Option<&[i32; 4]>,
    ) -> Result<MatchResult, PluginError> {
        let (template_path, threshold) = match recognition {
            Recognition::TemplateMatch {
                template,
                threshold,
            } => (template, threshold.unwrap_or(0.7)),
            _ => {
                return Err(PluginError::RecognitionFailed(
                    "Not a TemplateMatch recognition".to_string(),
                ))
            }
        };
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(PluginError::RecognitionFailed(format!(
                "TemplateMatch threshold must be between 0 and 1; received {threshold}"
            )));
        }

        let resolved_path = resource_resolver::resolve_image(template_path);
        let template_img = image::open(&resolved_path).map_err(|e| {
            PluginError::RecognitionFailed(format!(
                "Failed to load template '{}' (resolved: {}): {}",
                template_path,
                resolved_path.display(),
                e
            ))
        })?;

        let screenshot_rgb = screenshot.to_rgb8();
        let template_rgb = template_img.to_rgb8();

        let (sx, sy, sw, sh) =
            resolve_recognition_roi(roi, screenshot_rgb.width(), screenshot_rgb.height())?;

        let tw = template_rgb.width();
        let th = template_rgb.height();

        if tw == 0 || th == 0 {
            return Err(PluginError::RecognitionFailed(
                "Template image is empty".to_string(),
            ));
        }

        if tw > sw || th > sh {
            return Ok(MatchResult {
                matched: false,
                confidence: 0.0,
                box_region: None,
                text: None,
                candidates: Vec::new(),
                raw_candidate_count: Some(0),
                distinct_candidate_count: Some(0),
                selection_error: None,
            });
        }

        // Keep the complete eligible set. Candidate selection may request a
        // spatial ordering, so truncating to the strongest matches here would
        // make valid left/right/top/bottom indexes unreachable.
        let mut raw_detections = Vec::<RawDetection>::new();
        let pixel_count = tw as u64 * th as u64;
        let maximum_total_diff = (1.0 - threshold) * MAX_RGB_DIFF_SQ * pixel_count as f64;
        let mut scan_index = 0usize;

        let step = 1u32;
        let mut y = sy;
        while y + th <= sy + sh {
            let mut x = sx;
            while x + tw <= sx + sw {
                let mut total_diff = 0u64;
                let mut rejected = false;

                'compare: for ty in 0..th {
                    for tx in 0..tw {
                        let sp = screenshot_rgb.get_pixel(x + tx, y + ty);
                        let tp = template_rgb.get_pixel(tx, ty);

                        let dr = sp[0] as i32 - tp[0] as i32;
                        let dg = sp[1] as i32 - tp[1] as i32;
                        let db = sp[2] as i32 - tp[2] as i32;

                        total_diff += (dr * dr + dg * dg + db * db) as u64;
                        if total_diff as f64 > maximum_total_diff {
                            rejected = true;
                            break 'compare;
                        }
                    }
                }

                if !rejected {
                    let avg_diff = total_diff as f64 / pixel_count as f64;
                    raw_detections.push(RawDetection {
                        avg_diff,
                        x,
                        y,
                        scan_index,
                    });
                }
                scan_index += 1;

                x += step;
            }
            y += step;
        }

        // Template matching's native ordering is strongest match first.
        // Stable coordinate tie-breakers make identical scores deterministic.
        raw_detections.sort_by(|a, b| {
            a.avg_diff
                .total_cmp(&b.avg_diff)
                .then_with(|| a.y.cmp(&b.y))
                .then_with(|| a.x.cmp(&b.x))
                .then_with(|| a.scan_index.cmp(&b.scan_index))
        });
        let raw_candidate_count = raw_detections.len();
        let candidates = suppress_overlaps(raw_detections, tw, th, TEMPLATE_NMS_IOU);

        let match_candidates: Vec<MatchCandidate> = candidates
            .iter()
            .enumerate()
            .map(|(rank, detection)| {
                let confidence = 1.0 - (detection.avg_diff / MAX_RGB_DIFF_SQ);
                MatchCandidate {
                    rank,
                    region: Region {
                        x: detection.x as i32,
                        y: detection.y as i32,
                        width: tw,
                        height: th,
                    },
                    confidence,
                    text: None,
                    selected: false,
                }
            })
            .collect();

        let matched = !match_candidates.is_empty();
        let confidence = match_candidates
            .first()
            .map_or(0.0, |candidate| candidate.confidence);

        log::debug!(
            "[recognizer:TemplateMatch] template='{}', confidence={:.4}, matched={}, raw_candidates={}, distinct_candidates={}",
            template_path,
            confidence,
            matched,
            raw_candidate_count,
            match_candidates.len()
        );
        if raw_candidate_count > 10_000 {
            log::warn!(
                "[recognizer:TemplateMatch] template='{}' produced {} raw matches at threshold {:.3}; consider a tighter template or higher threshold",
                template_path,
                raw_candidate_count,
                threshold
            );
        }

        let distinct_candidate_count = match_candidates.len();
        Ok(MatchResult {
            matched,
            confidence,
            box_region: match_candidates.first().map(|c| c.region.clone()),
            text: None,
            candidates: match_candidates,
            raw_candidate_count: Some(raw_candidate_count),
            distinct_candidate_count: Some(distinct_candidate_count),
            selection_error: None,
        })
    }
}

use crate::modules::pipeline::authoring::models::recognizer::Recognition;

#[cfg(test)]
mod tests {
    use super::*;

    fn detection(x: u32, y: u32, avg_diff: f64, scan_index: usize) -> RawDetection {
        RawDetection {
            x,
            y,
            avg_diff,
            scan_index,
        }
    }

    #[test]
    fn overlap_suppression_keeps_the_strongest_detection() {
        let kept = suppress_overlaps(
            vec![detection(10, 10, 1.0, 0), detection(11, 10, 2.0, 1)],
            20,
            20,
            0.5,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!((kept[0].x, kept[0].y), (10, 10));
    }

    #[test]
    fn overlap_suppression_preserves_separate_objects() {
        let kept = suppress_overlaps(
            vec![detection(10, 10, 1.0, 0), detection(40, 10, 2.0, 1)],
            20,
            20,
            0.5,
        );
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn overlap_suppression_is_deterministic_for_equal_scores() {
        let mut candidates = vec![detection(11, 10, 1.0, 1), detection(10, 10, 1.0, 0)];
        candidates.sort_by(|a, b| {
            a.avg_diff
                .total_cmp(&b.avg_diff)
                .then_with(|| a.y.cmp(&b.y))
                .then_with(|| a.x.cmp(&b.x))
                .then_with(|| a.scan_index.cmp(&b.scan_index))
        });
        let kept = suppress_overlaps(candidates, 20, 20, 0.5);
        assert_eq!((kept[0].x, kept[0].y), (10, 10));
    }
}

use crate::modules::pipeline::models::recognizer::{MatchResult, Recognition, Region};
use crate::modules::pipeline::plugins::shared::resolve_recognition_roi;
use crate::modules::pipeline::plugins::{PluginError, RecognizerPlugin};
use image::DynamicImage;

pub struct ColorMatchRecognizer;

impl RecognizerPlugin for ColorMatchRecognizer {
    fn name(&self) -> &str {
        "ColorMatch"
    }

    fn evaluate(
        &self,
        screenshot: &DynamicImage,
        recognition: &Recognition,
        roi: Option<&[i32; 4]>,
    ) -> Result<MatchResult, PluginError> {
        let (lower, upper) = match recognition {
            Recognition::ColorMatch { lower, upper } => (lower, upper),
            _ => {
                return Err(PluginError::RecognitionFailed(
                    "Not a ColorMatch recognition".to_string(),
                ))
            }
        };

        if lower.len() < 3 || upper.len() < 3 {
            return Err(PluginError::RecognitionFailed(
                "ColorMatch lower/upper must have at least 3 RGB values".to_string(),
            ));
        }

        let rgb = screenshot.to_rgb8();
        let (sx, sy, sw, sh) = resolve_recognition_roi(roi, rgb.width(), rgb.height())?;

        let mut match_count = 0u64;
        let mut total_count = 0u64;
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        for y in sy..sy + sh {
            for x in sx..sx + sw {
                let p = rgb.get_pixel(x, y);
                let in_range = p[0] >= lower[0]
                    && p[0] <= upper[0]
                    && p[1] >= lower[1]
                    && p[1] <= upper[1]
                    && p[2] >= lower[2]
                    && p[2] <= upper[2];
                total_count += 1;
                if in_range {
                    match_count += 1;
                    if x < min_x {
                        min_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }

        let confidence = if total_count > 0 {
            match_count as f64 / total_count as f64
        } else {
            0.0
        };
        let matched = match_count > 0;

        log::debug!(
            "[recognizer:ColorMatch] pixels={}/{}, confidence={:.4}, matched={}",
            match_count,
            total_count,
            confidence,
            matched
        );

        Ok(MatchResult {
            matched,
            confidence,
            box_region: if matched && min_x != u32::MAX {
                Some(Region {
                    x: min_x as i32,
                    y: min_y as i32,
                    width: (max_x - min_x + 1),
                    height: (max_y - min_y + 1),
                })
            } else {
                None
            },
            text: None,
            candidates: Vec::new(),
            raw_candidate_count: None,
            distinct_candidate_count: None,
            selection_error: None,
        })
    }
}

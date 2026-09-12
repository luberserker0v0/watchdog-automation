use crate::modules::pipeline::models::constants::MAX_RGB_DIFF_SQ;
use crate::modules::pipeline::models::recognizer::{MatchCandidate, MatchResult, Region};
use crate::modules::pipeline::plugins::shared::resolve_recognition_roi;
use crate::modules::pipeline::plugins::{PluginError, RecognizerPlugin};
use crate::utils::resource_resolver;
use image::DynamicImage;

pub struct TemplateRecognizer;

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

        if tw > sw || th > sh {
            return Ok(MatchResult {
                matched: false,
                confidence: 0.0,
                box_region: None,
                text: None,
                candidates: Vec::new(),
                selection_error: None,
            });
        }

        // Keep the complete eligible set. Candidate selection may request a
        // spatial ordering, so truncating to the strongest matches here would
        // make valid left/right/top/bottom indexes unreachable.
        let mut candidates: Vec<(f64, u32, u32)> = Vec::new();

        let step = 1u32;
        let mut y = sy;
        while y + th <= sy + sh {
            let mut x = sx;
            while x + tw <= sx + sw {
                let mut total_diff = 0u64;
                let mut count = 0u64;

                for ty in 0..th {
                    for tx in 0..tw {
                        let sp = screenshot_rgb.get_pixel(x + tx, y + ty);
                        let tp = template_rgb.get_pixel(tx, ty);

                        let dr = sp[0] as i32 - tp[0] as i32;
                        let dg = sp[1] as i32 - tp[1] as i32;
                        let db = sp[2] as i32 - tp[2] as i32;

                        total_diff += (dr * dr + dg * dg + db * db) as u64;
                        count += 1;
                    }
                }

                let avg_diff = total_diff as f64 / count as f64;
                let confidence = 1.0 - (avg_diff / MAX_RGB_DIFF_SQ);
                if confidence >= threshold {
                    candidates.push((avg_diff, x, y));
                }

                x += step;
            }
            y += step;
        }

        // Template matching's native ordering is strongest match first.
        // Stable coordinate tie-breakers make identical scores deterministic.
        candidates.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| a.1.cmp(&b.1))
        });

        // Build MatchCandidate list with ranks
        let match_candidates: Vec<MatchCandidate> = candidates
            .iter()
            .enumerate()
            .map(|(rank, (score, x, y))| {
                let confidence = 1.0 - (score / MAX_RGB_DIFF_SQ);
                MatchCandidate {
                    rank,
                    region: Region {
                        x: *x as i32,
                        y: *y as i32,
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
            "[recognizer:TemplateMatch] template='{}', confidence={:.4}, matched={}, candidates={}",
            template_path,
            confidence,
            matched,
            match_candidates.len()
        );

        Ok(MatchResult {
            matched,
            confidence,
            box_region: match_candidates.first().map(|c| c.region.clone()),
            text: None,
            candidates: match_candidates,
            selection_error: None,
        })
    }
}

use crate::modules::pipeline::models::recognizer::Recognition;

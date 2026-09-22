use crate::modules::pipeline::authoring::models::recognizer::{
    MatchCandidate, MatchResult, Recognition, Region,
};
use crate::modules::pipeline::plugins::shared::resolve_recognition_roi;
use crate::modules::pipeline::plugins::{PluginError, RecognizerPlugin};
use image::DynamicImage;
use regex_lite::Regex;
use std::sync::Mutex;

use paddleocr_rs_onnx::{AccelerationDevice, OcrEngine};

struct OcrEngineState {
    engine: Option<OcrEngine>,
    device: AccelerationDevice,
}

static ENGINE: Mutex<OcrEngineState> = Mutex::new(OcrEngineState {
    engine: None,
    device: AccelerationDevice::Cpu,
});

const DET_MODEL: &[u8] = include_bytes!("../../../../../resources/models/ocr/det.onnx");
const REC_MODEL: &[u8] = include_bytes!("../../../../../resources/models/ocr/rec.onnx");
const KEYS_FILE: &[u8] = include_bytes!("../../../../../resources/models/ocr/keys.txt");

struct OcrTextMatcher {
    pattern: Regex,
    whitespace_tolerant_pattern: Option<Regex>,
}

impl OcrTextMatcher {
    fn new(pattern: &str) -> Result<Self, regex_lite::Error> {
        let whitespace_tolerant = make_literal_whitespace_optional(pattern);
        Ok(Self {
            pattern: Regex::new(pattern)?,
            whitespace_tolerant_pattern: (whitespace_tolerant != pattern)
                .then(|| Regex::new(&whitespace_tolerant))
                .transpose()?,
        })
    }

    fn is_match(&self, text: &str) -> bool {
        self.pattern.is_match(text)
            || self
                .whitespace_tolerant_pattern
                .as_ref()
                .is_some_and(|pattern| pattern.is_match(text))
    }
}

/// OCR models may omit a visible word gap from the decoded text. Make only
/// literal whitespace in a user pattern optional; explicit regex constructs
/// such as `\s+` and spaces inside character classes retain their semantics.
fn make_literal_whitespace_optional(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    let mut in_character_class = false;

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            result.push(ch);
            if let Some(escaped) = chars.next() {
                result.push(escaped);
            }
            continue;
        }

        match ch {
            '[' => {
                in_character_class = true;
                result.push(ch);
            }
            ']' if in_character_class => {
                in_character_class = false;
                result.push(ch);
            }
            _ if ch.is_whitespace() && !in_character_class => {
                while chars.peek().is_some_and(|next| next.is_whitespace()) {
                    chars.next();
                }
                result.push_str(r"\s*");
            }
            _ => result.push(ch),
        }
    }

    result
}

fn init_ort_runtime() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|e| format!("Failed to locate executable for ONNX Runtime: {}", e))?;
    let executable_dir = executable
        .parent()
        .ok_or_else(|| "Failed to locate executable directory for ONNX Runtime".to_string())?;
    let candidates = [
        executable_dir.join("onnxruntime.dll"),
        executable_dir
            .parent()
            .map(|parent| parent.join("onnxruntime.dll"))
            .unwrap_or_default(),
    ];
    let runtime_path = candidates
        .iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "ONNX Runtime not found. Searched: {}",
                candidates
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    ort::init_from(runtime_path)
        .map_err(|e| {
            format!(
                "Failed to load ONNX Runtime from {}: {}",
                runtime_path.display(),
                e
            )
        })?
        .commit();
    Ok(())
}

fn create_engine(requested: AccelerationDevice) -> Result<(OcrEngine, AccelerationDevice), String> {
    let selected = if requested == AccelerationDevice::DirectML && !requested.is_available() {
        log::warn!("[ocr] DirectML is unavailable; falling back to CPU");
        AccelerationDevice::Cpu
    } else {
        requested
    };

    match OcrEngine::new_with_device(DET_MODEL, REC_MODEL, KEYS_FILE, selected) {
        Ok(engine) => Ok((engine, selected)),
        Err(gpu_error) if selected != AccelerationDevice::Cpu => {
            log::warn!(
                "[ocr] failed to initialize {}: {:?}; falling back to CPU",
                selected,
                gpu_error
            );
            OcrEngine::new(DET_MODEL, REC_MODEL, KEYS_FILE)
                .map(|engine| (engine, AccelerationDevice::Cpu))
                .map_err(|cpu_error| {
                    format!(
                        "Failed to initialize OCR on {} ({:?}) or CPU ({:?})",
                        selected, gpu_error, cpu_error
                    )
                })
        }
        Err(error) => Err(format!("Failed to init PaddleOCR engine: {:?}", error)),
    }
}

fn get_engine() -> Result<std::sync::MutexGuard<'static, OcrEngineState>, String> {
    let mut guard = ENGINE
        .lock()
        .map_err(|e| format!("Engine lock poisoned: {}", e))?;

    if guard.engine.is_none() {
        init_ort_runtime()?;
        let (engine, device) = create_engine(guard.device)?;
        guard.engine = Some(engine);
        guard.device = device;
        log::info!("PaddleOCR engine initialized on {}", device);
    }

    Ok(guard)
}

pub fn set_device(value: &str) -> Result<String, String> {
    let requested = match value.to_ascii_lowercase().as_str() {
        "cpu" => AccelerationDevice::Cpu,
        "directml" | "gpu" => AccelerationDevice::DirectML,
        _ => return Err(format!("Unsupported OCR device '{}'", value)),
    };

    init_ort_runtime()?;
    let mut guard = ENGINE
        .lock()
        .map_err(|e| format!("Engine lock poisoned: {}", e))?;
    if guard.engine.is_some() && guard.device == requested {
        return Ok(guard.device.to_string());
    }

    log::info!("[ocr] switching recognition device to {}", requested);
    let (engine, active_device) = create_engine(requested)?;
    guard.engine = Some(engine);
    guard.device = active_device;
    Ok(active_device.to_string())
}

pub struct OcrRecognizer;

#[derive(Debug, Clone)]
struct RecognizedTextBlock {
    text: String,
    region: Region,
    confidence: f64,
}

fn union_region(blocks: &[RecognizedTextBlock]) -> Region {
    let left = blocks.iter().map(|block| block.region.x).min().unwrap_or(0);
    let top = blocks.iter().map(|block| block.region.y).min().unwrap_or(0);
    let right = blocks
        .iter()
        .map(|block| block.region.x + block.region.width as i32)
        .max()
        .unwrap_or(left);
    let bottom = blocks
        .iter()
        .map(|block| block.region.y + block.region.height as i32)
        .max()
        .unwrap_or(top);
    Region {
        x: left,
        y: top,
        width: right.saturating_sub(left) as u32,
        height: bottom.saturating_sub(top) as u32,
    }
}

fn compact_text_group(blocks: &[RecognizedTextBlock]) -> bool {
    let union = union_region(blocks);
    let union_area = u64::from(union.width) * u64::from(union.height);
    let text_area = blocks
        .iter()
        .map(|block| u64::from(block.region.width) * u64::from(block.region.height))
        .sum::<u64>();
    text_area > 0 && union_area <= text_area.saturating_mul(8)
}

/// PaddleOCR may return each wrapped word as its own block. When no individual
/// block satisfies the expected pattern, form the smallest matching sequence
/// of nearby blocks so displayed aggregate text and match behavior agree.
fn composite_text_candidates(
    blocks: &[RecognizedTextBlock],
    matcher: &OcrTextMatcher,
    threshold: f64,
) -> Vec<MatchCandidate> {
    const MAX_COMPOSITE_BLOCKS: usize = 8;
    for span in 2..=blocks.len().min(MAX_COMPOSITE_BLOCKS) {
        let mut candidates = Vec::new();
        for window in blocks.windows(span) {
            if !compact_text_group(window) {
                continue;
            }
            let text = window
                .iter()
                .map(|block| block.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let confidence =
                window.iter().map(|block| block.confidence).sum::<f64>() / window.len() as f64;
            if matcher.is_match(&text) && confidence >= threshold {
                candidates.push(MatchCandidate {
                    rank: candidates.len(),
                    region: union_region(window),
                    confidence,
                    text: Some(text),
                    selected: false,
                });
            }
        }
        if !candidates.is_empty() {
            return candidates;
        }
    }
    Vec::new()
}

impl RecognizerPlugin for OcrRecognizer {
    fn name(&self) -> &str {
        "OCR"
    }

    fn evaluate(
        &self,
        screenshot: &DynamicImage,
        recognition: &Recognition,
        roi: Option<&[i32; 4]>,
    ) -> Result<MatchResult, PluginError> {
        let (expected, threshold) = match recognition {
            Recognition::Ocr {
                expected,
                threshold,
            } => (expected.as_deref(), threshold.unwrap_or(0.3)),
            _ => {
                return Err(PluginError::RecognitionFailed(
                    "Not an OCR recognition".to_string(),
                ))
            }
        };

        let guard = get_engine().map_err(PluginError::RecognitionFailed)?;
        let engine = guard.engine.as_ref().ok_or_else(|| {
            PluginError::RecognitionFailed("OCR engine not initialized".to_string())
        })?;

        let (sx, sy, sw, sh) =
            resolve_recognition_roi(roi, screenshot.width(), screenshot.height())?;

        let full_rgb = screenshot.to_rgb8();
        let raw = full_rgb.as_raw();

        let mut cropped = image::RgbImage::new(sw, sh);
        for row in 0..sh {
            let src_offset = ((sy + row) * full_rgb.width() + sx) as usize * 3;
            let dst_offset = (row * sw) as usize * 3;
            let len = sw as usize * 3;
            let src_slice = &raw[src_offset..src_offset + len];
            let dst_slice = &mut cropped.as_mut()[dst_offset..dst_offset + len];
            dst_slice.copy_from_slice(src_slice);
        }

        let cropped_image = DynamicImage::ImageRgb8(cropped);

        let blocks = engine
            .recognize_all(&cropped_image, paddleocr_rs_onnx::OrderBy::Horizontal)
            .map_err(|e| {
                PluginError::RecognitionFailed(format!("PaddleOCR recognition failed: {:?}", e))
            })?;

        let mut recognized_blocks = Vec::new();
        let mut candidates: Vec<MatchCandidate> = Vec::new();

        let compiled_re = if let Some(pattern) = expected {
            Some(OcrTextMatcher::new(pattern).map_err(|e| {
                PluginError::RecognitionFailed(format!("Invalid regex '{}': {}", pattern, e))
            })?)
        } else {
            None
        };

        for block in &blocks {
            let text = block.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            let confidence = block.confidence as f64;
            let region = Region {
                x: sx as i32 + block.x as i32,
                y: sy as i32 + block.y as i32,
                width: block.width as u32,
                height: block.height as u32,
            };
            recognized_blocks.push(RecognizedTextBlock {
                text: text.clone(),
                region: region.clone(),
                confidence,
            });

            let block_matched = if let Some(re) = &compiled_re {
                re.is_match(&text)
            } else {
                true
            };

            if block_matched && confidence >= threshold {
                candidates.push(MatchCandidate {
                    rank: candidates.len(),
                    region,
                    confidence,
                    text: Some(text),
                    selected: false,
                });
            }
        }

        if candidates.is_empty() {
            if let Some(matcher) = &compiled_re {
                candidates = composite_text_candidates(&recognized_blocks, matcher, threshold);
            }
        }

        // Assign final ranks
        for (i, c) in candidates.iter_mut().enumerate() {
            c.rank = i;
        }

        let combined_text = recognized_blocks
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let selected = candidates.first().cloned();
        let matched = selected.is_some();
        let confidence = selected
            .as_ref()
            .map_or(0.0, |candidate| candidate.confidence);

        Ok(MatchResult {
            matched,
            confidence,
            box_region: selected.map(|candidate| candidate.region),
            text: Some(combined_text),
            candidates,
            raw_candidate_count: None,
            distinct_candidate_count: None,
            selection_error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        composite_text_candidates, make_literal_whitespace_optional, OcrTextMatcher,
        RecognizedTextBlock,
    };
    use crate::modules::pipeline::authoring::models::recognizer::Region;

    #[test]
    fn literal_space_matches_ocr_text_with_or_without_the_gap() {
        let matcher = OcrTextMatcher::new(r"^Card 1$").unwrap();

        assert!(matcher.is_match("Card 1"));
        assert!(matcher.is_match("Card1"));
        assert!(!matcher.is_match("Card 10"));
    }

    #[test]
    fn explicit_regex_whitespace_keeps_its_semantics() {
        let matcher = OcrTextMatcher::new(r"^Card\s+1$").unwrap();

        assert!(matcher.is_match("Card 1"));
        assert!(!matcher.is_match("Card1"));
    }

    #[test]
    fn whitespace_inside_character_class_is_not_rewritten() {
        assert_eq!(
            make_literal_whitespace_optional(r"^Card[ ]1$"),
            r"^Card[ ]1$"
        );
    }

    #[test]
    fn wrapped_words_form_one_bounded_candidate() {
        let blocks = [
            ("Arm", 10, 10, 30, 12, 0.91),
            ("first", 8, 25, 35, 12, 0.87),
            ("miss", 9, 40, 32, 12, 0.93),
        ]
        .into_iter()
        .map(
            |(text, x, y, width, height, confidence)| RecognizedTextBlock {
                text: text.into(),
                region: Region {
                    x,
                    y,
                    width,
                    height,
                },
                confidence,
            },
        )
        .collect::<Vec<_>>();

        let candidates = composite_text_candidates(
            &blocks,
            &OcrTextMatcher::new("^Arm first miss$").unwrap(),
            0.6,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].text.as_deref(), Some("Arm first miss"));
        assert_eq!(
            candidates[0].region,
            Region {
                x: 8,
                y: 10,
                width: 35,
                height: 42
            }
        );
        assert!((candidates[0].confidence - 0.9033).abs() < 0.001);
    }

    #[test]
    fn distant_words_do_not_form_an_action_candidate() {
        let blocks = vec![
            RecognizedTextBlock {
                text: "Arm".into(),
                region: Region {
                    x: 0,
                    y: 0,
                    width: 30,
                    height: 12,
                },
                confidence: 0.9,
            },
            RecognizedTextBlock {
                text: "first".into(),
                region: Region {
                    x: 300,
                    y: 300,
                    width: 35,
                    height: 12,
                },
                confidence: 0.9,
            },
            RecognizedTextBlock {
                text: "miss".into(),
                region: Region {
                    x: 600,
                    y: 600,
                    width: 32,
                    height: 12,
                },
                confidence: 0.9,
            },
        ];

        assert!(composite_text_candidates(
            &blocks,
            &OcrTextMatcher::new("Arm first miss").unwrap(),
            0.6,
        )
        .is_empty());
    }

    #[test]
    #[ignore = "requires the installed ONNX Runtime and initializes OCR models"]
    fn directml_device_initializes_or_falls_back_to_cpu() {
        let active = super::set_device("directml").unwrap();
        println!("active OCR device: {}", active);
        assert!(active == "DirectML" || active == "CPU");
    }
}

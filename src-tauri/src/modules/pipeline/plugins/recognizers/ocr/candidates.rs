use crate::modules::pipeline::authoring::models::recognizer::{MatchCandidate, Region};

use super::matcher::OcrTextMatcher;
#[derive(Debug, Clone)]
pub(super) struct RecognizedTextBlock {
    pub(super) text: String,
    pub(super) region: Region,
    pub(super) confidence: f64,
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
pub(super) fn composite_text_candidates(
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

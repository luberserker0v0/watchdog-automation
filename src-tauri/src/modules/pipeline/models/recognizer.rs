use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionConfig {
    #[serde(flatten)]
    pub kind: Recognition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roi: Option<RecognitionRoi>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roi_offset: Option<[i32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_selection: Option<CandidateSelection>,
}

/// A fixed capture-local rectangle or the recognized box of an earlier node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecognitionRoi {
    Fixed([i32; 4]),
    Node(String),
}

impl Default for RecognitionConfig {
    fn default() -> Self {
        Self {
            kind: Recognition::DirectHit,
            roi: None,
            roi_offset: None,
            candidate_selection: None,
        }
    }
}

impl From<Recognition> for RecognitionConfig {
    fn from(kind: Recognition) -> Self {
        Self {
            kind,
            roi: None,
            roi_offset: None,
            candidate_selection: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateOrder {
    #[default]
    Native,
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
    ConfidenceDesc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSelection {
    #[serde(default)]
    pub order: CandidateOrder,
    #[serde(default)]
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Recognition {
    #[serde(rename = "DirectHit")]
    DirectHit,

    #[serde(rename = "TemplateMatch")]
    TemplateMatch {
        template: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f64>,
    },

    #[serde(rename = "OCR")]
    Ocr {
        #[serde(skip_serializing_if = "Option::is_none")]
        expected: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f64>,
    },

    #[serde(rename = "ColorMatch")]
    ColorMatch { lower: Vec<u8>, upper: Vec<u8> },

    #[serde(rename = "Custom")]
    Custom {
        recognition: String,
        #[serde(default)]
        param: serde_json::Value,
    },
}

impl Recognition {
    pub fn substitute(
        &self,
        params: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        if params.is_empty() {
            return self.clone();
        }
        match self.clone() {
            Recognition::Ocr {
                expected,
                threshold,
            } => Recognition::Ocr {
                expected: expected.map(|e| substitute_string(&e, params)),
                threshold,
            },
            Recognition::Custom { recognition, param } => Recognition::Custom {
                recognition,
                param: substitute_json(&param, params),
            },
            other => other,
        }
    }
}

fn substitute_string(
    s: &str,
    params: &std::collections::HashMap<String, serde_json::Value>,
) -> String {
    let mut result = s.to_string();
    for (key, value) in params {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            serde_json::Value::String(v) => v.clone(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Null => "null".to_string(),
            _ => value.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

fn substitute_json(
    value: &serde_json::Value,
    params: &std::collections::HashMap<String, serde_json::Value>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => serde_json::Value::String(substitute_string(s, params)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| substitute_json(v, params)).collect())
        }
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), substitute_json(v, params)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    pub fn center(&self) -> (i32, i32) {
        (
            self.x + self.width as i32 / 2,
            self.y + self.height as i32 / 2,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchCandidate {
    pub rank: usize,
    pub region: Region,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default)]
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub matched: bool,
    pub confidence: f64,
    pub box_region: Option<Region>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default)]
    pub candidates: Vec<MatchCandidate>,
    /// Number of threshold-passing detections before recognizer-specific
    /// consolidation such as template overlap suppression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_candidate_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct_candidate_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_error: Option<String>,
}

impl Default for MatchResult {
    fn default() -> Self {
        Self {
            matched: false,
            confidence: 0.0,
            box_region: None,
            text: None,
            candidates: Vec::new(),
            raw_candidate_count: None,
            distinct_candidate_count: None,
            selection_error: None,
        }
    }
}

pub fn effective_match(result: &MatchResult, inverse: bool) -> bool {
    if result.selection_error.is_some() {
        return false;
    }
    if inverse {
        !result.matched
    } else {
        result.matched
    }
}

pub fn select_candidate(
    mut candidates: Vec<MatchCandidate>,
    selection: Option<&CandidateSelection>,
) -> (Option<MatchCandidate>, Vec<MatchCandidate>, Option<String>) {
    let selection = selection.cloned().unwrap_or_default();
    for (native_index, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = native_index;
        candidate.selected = false;
    }

    let center_x = |candidate: &MatchCandidate| candidate.region.center().0;
    let center_y = |candidate: &MatchCandidate| candidate.region.center().1;
    let native_rank = |a: &MatchCandidate, b: &MatchCandidate| a.rank.cmp(&b.rank);
    candidates.sort_by(|a, b| {
        let primary = match selection.order {
            CandidateOrder::Native => native_rank(a, b),
            CandidateOrder::TopToBottom => center_y(a)
                .cmp(&center_y(b))
                .then_with(|| center_x(a).cmp(&center_x(b))),
            CandidateOrder::BottomToTop => center_y(b)
                .cmp(&center_y(a))
                .then_with(|| center_x(a).cmp(&center_x(b))),
            CandidateOrder::LeftToRight => center_x(a)
                .cmp(&center_x(b))
                .then_with(|| center_y(a).cmp(&center_y(b))),
            CandidateOrder::RightToLeft => center_x(b)
                .cmp(&center_x(a))
                .then_with(|| center_y(a).cmp(&center_y(b))),
            CandidateOrder::ConfidenceDesc => b
                .confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(Ordering::Equal),
        };
        primary.then_with(|| native_rank(a, b))
    });

    for (rank, candidate) in candidates.iter_mut().enumerate() {
        candidate.rank = rank;
    }
    if selection.index >= candidates.len() {
        let error = format!(
            "Candidate index {} is unavailable; {} eligible candidate(s)",
            selection.index,
            candidates.len()
        );
        return (None, candidates, Some(error));
    }

    candidates[selection.index].selected = true;
    (Some(candidates[selection.index].clone()), candidates, None)
}

pub fn limit_debug_candidates(candidates: &mut Vec<MatchCandidate>, maximum: usize) {
    if candidates.len() <= maximum {
        return;
    }
    let selected = candidates
        .iter()
        .find(|candidate| candidate.selected)
        .cloned();
    candidates.truncate(maximum);
    if let Some(selected) = selected {
        if !candidates.iter().any(|candidate| candidate.selected) {
            if let Some(last) = candidates.last_mut() {
                *last = selected;
            } else if maximum > 0 {
                candidates.push(selected);
            }
        }
    }
}

#[cfg(test)]
mod tests;

pub mod actions;
pub mod recognizers;
pub mod shared;

use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::recognizer::{
    select_candidate, CandidateSelection, MatchResult, Recognition,
};
use image::DynamicImage;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum PluginError {
    RecognitionFailed(String),
    ActionFailed(String),
    NotFound(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::RecognitionFailed(msg) => write!(f, "Recognition failed: {}", msg),
            PluginError::ActionFailed(msg) => write!(f, "Action failed: {}", msg),
            PluginError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

pub trait RecognizerPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn evaluate(
        &self,
        screenshot: &DynamicImage,
        recognition: &Recognition,
        roi: Option<&[i32; 4]>,
    ) -> Result<MatchResult, PluginError>;
}

#[async_trait::async_trait]
pub trait ActionPlugin: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(
        &self,
        action: &Action,
        box_region: Option<&crate::modules::pipeline::models::recognizer::Region>,
        prev_regions: &HashMap<String, crate::modules::pipeline::models::recognizer::Region>,
    ) -> Result<(), PluginError>;
}

pub struct PluginRegistry {
    recognizers: HashMap<String, Box<dyn RecognizerPlugin>>,
    actions: HashMap<String, Box<dyn ActionPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            recognizers: HashMap::new(),
            actions: HashMap::new(),
        }
    }

    pub fn register_recognizer(&mut self, name: String, plugin: Box<dyn RecognizerPlugin>) {
        self.recognizers.insert(name, plugin);
    }

    pub fn register_action(&mut self, name: String, plugin: Box<dyn ActionPlugin>) {
        self.actions.insert(name, plugin);
    }

    pub fn evaluate_recognition(
        &self,
        screenshot: &DynamicImage,
        recognition: &Recognition,
        roi: Option<&[i32; 4]>,
    ) -> Result<MatchResult, PluginError> {
        self.evaluate_recognition_with_selection(screenshot, recognition, roi, None)
    }

    pub fn evaluate_recognition_with_selection(
        &self,
        screenshot: &DynamicImage,
        recognition: &Recognition,
        roi: Option<&[i32; 4]>,
        selection: Option<&CandidateSelection>,
    ) -> Result<MatchResult, PluginError> {
        let mut result = match recognition {
            Recognition::DirectHit => {
                let (x, y, width, height) =
                    shared::resolve_recognition_roi(roi, screenshot.width(), screenshot.height())?;
                Ok(MatchResult {
                    matched: true,
                    confidence: 1.0,
                    box_region: Some(crate::modules::pipeline::models::recognizer::Region {
                        x: x as i32,
                        y: y as i32,
                        width,
                        height,
                    }),
                    text: None,
                    candidates: Vec::new(),
                    selection_error: None,
                })
            }
            Recognition::TemplateMatch { .. } => {
                if let Some(plugin) = self.recognizers.get("TemplateMatch") {
                    plugin.evaluate(screenshot, recognition, roi)
                } else {
                    Err(PluginError::NotFound(
                        "TemplateMatch plugin not registered".to_string(),
                    ))
                }
            }
            Recognition::Ocr { .. } => {
                if let Some(plugin) = self.recognizers.get("OCR") {
                    plugin.evaluate(screenshot, recognition, roi)
                } else {
                    Err(PluginError::NotFound(
                        "OCR plugin not registered".to_string(),
                    ))
                }
            }
            Recognition::ColorMatch { .. } => {
                if let Some(plugin) = self.recognizers.get("ColorMatch") {
                    plugin.evaluate(screenshot, recognition, roi)
                } else {
                    Err(PluginError::NotFound(
                        "ColorMatch plugin not registered".to_string(),
                    ))
                }
            }
            Recognition::Custom {
                recognition: name, ..
            } => {
                if let Some(plugin) = self.recognizers.get(name.as_str()) {
                    plugin.evaluate(screenshot, recognition, roi)
                } else {
                    Err(PluginError::NotFound(format!(
                        "Custom recognition '{}' not registered",
                        name
                    )))
                }
            }
        }?;

        let supports_candidate_selection = matches!(
            recognition,
            Recognition::TemplateMatch { .. }
                | Recognition::Ocr { .. }
                | Recognition::Custom { .. }
        );
        if supports_candidate_selection && (!result.candidates.is_empty() || selection.is_some()) {
            let (selected, candidates, selection_error) =
                select_candidate(result.candidates, selection);
            result.matched = selected.is_some();
            result.confidence = selected
                .as_ref()
                .map_or(0.0, |candidate| candidate.confidence);
            result.box_region = selected.map(|candidate| candidate.region);
            result.candidates = candidates;
            result.selection_error = selection_error;
        }
        Ok(result)
    }

    pub async fn execute_action(
        &self,
        action: &Action,
        box_region: Option<&crate::modules::pipeline::models::recognizer::Region>,
        prev_regions: &HashMap<String, crate::modules::pipeline::models::recognizer::Region>,
    ) -> Result<(), PluginError> {
        match action {
            Action::DoNothing => Ok(()),
            Action::Click { .. } => {
                if let Some(plugin) = self.actions.get("Click") {
                    plugin.execute(action, box_region, prev_regions).await
                } else {
                    Err(PluginError::NotFound(
                        "Click plugin not registered".to_string(),
                    ))
                }
            }
            Action::LongPress { .. } => {
                if let Some(plugin) = self.actions.get("LongPress") {
                    plugin.execute(action, box_region, prev_regions).await
                } else {
                    Err(PluginError::NotFound(
                        "LongPress plugin not registered".to_string(),
                    ))
                }
            }
            Action::Swipe { .. } => {
                if let Some(plugin) = self.actions.get("Swipe") {
                    plugin.execute(action, box_region, prev_regions).await
                } else {
                    Err(PluginError::NotFound(
                        "Swipe plugin not registered".to_string(),
                    ))
                }
            }
            Action::Scroll { .. } => {
                if let Some(plugin) = self.actions.get("Scroll") {
                    plugin.execute(action, box_region, prev_regions).await
                } else {
                    Err(PluginError::NotFound(
                        "Scroll plugin not registered".to_string(),
                    ))
                }
            }
            Action::ClickKey { .. } => {
                if let Some(plugin) = self.actions.get("ClickKey") {
                    plugin.execute(action, box_region, prev_regions).await
                } else {
                    Err(PluginError::NotFound(
                        "ClickKey plugin not registered".to_string(),
                    ))
                }
            }
            Action::InputText { .. } => {
                if let Some(plugin) = self.actions.get("InputText") {
                    plugin.execute(action, box_region, prev_regions).await
                } else {
                    Err(PluginError::NotFound(
                        "InputText plugin not registered".to_string(),
                    ))
                }
            }
            Action::StopTask => Ok(()),
            Action::Custom { action: name, .. } => {
                if let Some(plugin) = self.actions.get(name.as_str()) {
                    plugin.execute(action, box_region, prev_regions).await
                } else {
                    Err(PluginError::NotFound(format!(
                        "Custom action '{}' not registered",
                        name
                    )))
                }
            }
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

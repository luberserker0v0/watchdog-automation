use crate::modules::pipeline::authoring::models::action::Action;
use crate::modules::pipeline::authoring::models::recognizer::Region;
use crate::modules::pipeline::plugins::{ActionPlugin, PluginError};
use std::collections::HashMap;

pub struct InputTextAction;

#[async_trait::async_trait]
impl ActionPlugin for InputTextAction {
    fn name(&self) -> &str {
        "InputText"
    }

    async fn execute(
        &self,
        action: &Action,
        _box_region: Option<&Region>,
        _prev_regions: &HashMap<String, Region>,
    ) -> Result<(), PluginError> {
        let text = match action {
            Action::InputText { input_text } => input_text,
            _ => {
                return Err(PluginError::ActionFailed(
                    "Not an InputText action".to_string(),
                ))
            }
        };

        log::debug!(
            "[action:InputText] inputting '{}' (len={})",
            text,
            text.len()
        );

        let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| PluginError::ActionFailed(format!("Failed to create enigo: {}", e)))?;

        use enigo::Keyboard;
        enigo
            .text(text)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to type text: {}", e)))?;

        Ok(())
    }
}

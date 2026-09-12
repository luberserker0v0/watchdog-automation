use super::key_map::map_key;
use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::recognizer::Region;
use crate::modules::pipeline::plugins::{ActionPlugin, PluginError};
use std::collections::HashMap;

pub struct ClickKeyAction;

#[async_trait::async_trait]
impl ActionPlugin for ClickKeyAction {
    fn name(&self) -> &str {
        "ClickKey"
    }

    async fn execute(
        &self,
        action: &Action,
        _box_region: Option<&Region>,
        _prev_regions: &HashMap<String, Region>,
    ) -> Result<(), PluginError> {
        let keys = match action {
            Action::ClickKey { key } => key,
            _ => {
                return Err(PluginError::ActionFailed(
                    "Not a ClickKey action".to_string(),
                ))
            }
        };

        log::debug!("[action:ClickKey] pressing keys {:?}", keys);

        let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| PluginError::ActionFailed(format!("Failed to create enigo: {}", e)))?;

        use enigo::Keyboard;
        for key_code in keys {
            let key = map_key(*key_code).ok_or_else(|| {
                PluginError::ActionFailed(format!("Unknown key code: {}", key_code))
            })?;

            enigo
                .key(key, enigo::Direction::Click)
                .map_err(|e| PluginError::ActionFailed(format!("Failed to press key: {}", e)))?;
        }

        Ok(())
    }
}

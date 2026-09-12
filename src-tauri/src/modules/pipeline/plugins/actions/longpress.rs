use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::recognizer::Region;
use crate::modules::pipeline::plugins::shared::resolve_target_with_offset;
use crate::modules::pipeline::plugins::{ActionPlugin, PluginError};
use std::collections::HashMap;

pub struct LongPressAction;

#[async_trait::async_trait]
impl ActionPlugin for LongPressAction {
    fn name(&self) -> &str {
        "LongPress"
    }

    async fn execute(
        &self,
        action: &Action,
        box_region: Option<&Region>,
        prev_regions: &HashMap<String, Region>,
    ) -> Result<(), PluginError> {
        let (target, target_offset, duration) = match action {
            Action::LongPress {
                target,
                target_offset,
                duration,
            } => (target, target_offset, duration),
            _ => {
                return Err(PluginError::ActionFailed(
                    "Not a LongPress action".to_string(),
                ))
            }
        };

        let (final_x, final_y) =
            resolve_target_with_offset(target, target_offset.as_ref(), box_region, prev_regions)?;

        log::debug!(
            "[action:LongPress] pressing at ({}, {}) for {}ms",
            final_x,
            final_y,
            duration
        );

        let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| PluginError::ActionFailed(format!("Failed to create enigo: {}", e)))?;

        use enigo::Mouse;
        enigo
            .move_mouse(final_x, final_y, enigo::Coordinate::Abs)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to move mouse: {}", e)))?;

        enigo
            .button(enigo::Button::Left, enigo::Direction::Press)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to press button: {}", e)))?;

        tokio::time::sleep(std::time::Duration::from_millis(*duration)).await;

        enigo
            .button(enigo::Button::Left, enigo::Direction::Release)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to release button: {}", e)))?;

        Ok(())
    }
}

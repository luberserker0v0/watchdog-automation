use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::recognizer::Region;
use crate::modules::pipeline::plugins::shared::resolve_target_with_offset;
use crate::modules::pipeline::plugins::{ActionPlugin, PluginError};
use std::collections::HashMap;

pub struct ClickAction;

#[async_trait::async_trait]
impl ActionPlugin for ClickAction {
    fn name(&self) -> &str {
        "Click"
    }

    async fn execute(
        &self,
        action: &Action,
        box_region: Option<&Region>,
        prev_regions: &HashMap<String, Region>,
    ) -> Result<(), PluginError> {
        let (target, target_offset) = match action {
            Action::Click {
                target,
                target_offset,
            } => (target, target_offset),
            _ => return Err(PluginError::ActionFailed("Not a Click action".to_string())),
        };

        let (final_x, final_y) =
            resolve_target_with_offset(target, target_offset.as_ref(), box_region, prev_regions)?;

        log::debug!("[action:Click] executing at ({}, {})", final_x, final_y);

        let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| PluginError::ActionFailed(format!("Failed to create enigo: {}", e)))?;

        use enigo::Mouse;
        enigo
            .move_mouse(final_x, final_y, enigo::Coordinate::Abs)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to move mouse: {}", e)))?;

        enigo
            .button(enigo::Button::Left, enigo::Direction::Click)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to click: {}", e)))?;

        Ok(())
    }
}

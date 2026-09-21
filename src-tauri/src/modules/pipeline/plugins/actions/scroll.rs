use crate::modules::pipeline::authoring::models::action::Action;
use crate::modules::pipeline::authoring::models::recognizer::Region;
use crate::modules::pipeline::plugins::shared::resolve_target_with_offset;
use crate::modules::pipeline::plugins::{ActionPlugin, PluginError};
use std::collections::HashMap;

pub struct ScrollAction;

#[async_trait::async_trait]
impl ActionPlugin for ScrollAction {
    fn name(&self) -> &str {
        "Scroll"
    }

    async fn execute(
        &self,
        action: &Action,
        box_region: Option<&Region>,
        prev_regions: &HashMap<String, Region>,
    ) -> Result<(), PluginError> {
        let (target, target_offset, dx, dy) = match action {
            Action::Scroll {
                target,
                target_offset,
                dx,
                dy,
            } => (target, target_offset, dx, dy),
            _ => return Err(PluginError::ActionFailed("Not a Scroll action".to_string())),
        };

        let (final_x, final_y) =
            resolve_target_with_offset(target, target_offset.as_ref(), box_region, prev_regions)?;

        log::debug!(
            "[action:Scroll] scrolling at ({}, {}) dx={}, dy={}",
            final_x,
            final_y,
            dx,
            dy
        );

        let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| PluginError::ActionFailed(format!("Failed to create enigo: {}", e)))?;

        use enigo::{Axis, Mouse};
        enigo
            .move_mouse(final_x, final_y, enigo::Coordinate::Abs)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to move mouse: {}", e)))?;

        if *dx != 0 {
            enigo
                .scroll(*dx, Axis::Horizontal)
                .map_err(|e| PluginError::ActionFailed(format!("Failed to scroll: {}", e)))?;
        }

        if *dy != 0 {
            enigo
                .scroll(*dy, Axis::Vertical)
                .map_err(|e| PluginError::ActionFailed(format!("Failed to scroll: {}", e)))?;
        }

        Ok(())
    }
}

use crate::modules::pipeline::authoring::models::action::Action;
use crate::modules::pipeline::authoring::models::recognizer::Region;
use crate::modules::pipeline::plugins::shared::resolve_target_with_offset;
use crate::modules::pipeline::plugins::{ActionPlugin, PluginError};
use std::collections::HashMap;

pub struct SwipeAction;

#[async_trait::async_trait]
impl ActionPlugin for SwipeAction {
    fn name(&self) -> &str {
        "Swipe"
    }

    async fn execute(
        &self,
        action: &Action,
        box_region: Option<&Region>,
        prev_regions: &HashMap<String, Region>,
    ) -> Result<(), PluginError> {
        let (begin, begin_offset, end, end_offset, duration) = match action {
            Action::Swipe {
                begin,
                begin_offset,
                end,
                end_offset,
                duration,
            } => (begin, begin_offset, end, end_offset, duration),
            _ => return Err(PluginError::ActionFailed("Not a Swipe action".to_string())),
        };

        let (start_x, start_y) =
            resolve_target_with_offset(begin, begin_offset.as_ref(), box_region, prev_regions)?;
        let (end_x, end_y) =
            resolve_target_with_offset(end, end_offset.as_ref(), box_region, prev_regions)?;

        log::debug!(
            "[action:Swipe] swiping from ({}, {}) to ({}, {}) over {}ms",
            start_x,
            start_y,
            end_x,
            end_y,
            duration
        );

        let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| PluginError::ActionFailed(format!("Failed to create enigo: {}", e)))?;

        use enigo::Mouse;

        enigo
            .move_mouse(start_x, start_y, enigo::Coordinate::Abs)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to move mouse: {}", e)))?;

        enigo
            .button(enigo::Button::Left, enigo::Direction::Press)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to press button: {}", e)))?;

        let steps = 20;
        let step_duration = duration / steps;
        let dx = (end_x - start_x) as f64 / steps as f64;
        let dy = (end_y - start_y) as f64 / steps as f64;

        for i in 1..=steps {
            let x = start_x + (dx * i as f64) as i32;
            let y = start_y + (dy * i as f64) as i32;
            enigo
                .move_mouse(x, y, enigo::Coordinate::Abs)
                .map_err(|e| PluginError::ActionFailed(format!("Failed to move mouse: {}", e)))?;
            tokio::time::sleep(std::time::Duration::from_millis(step_duration)).await;
        }

        enigo
            .button(enigo::Button::Left, enigo::Direction::Release)
            .map_err(|e| PluginError::ActionFailed(format!("Failed to release button: {}", e)))?;

        Ok(())
    }
}

use kani_volcano_engine::prelude::*;
use anyhow::{Result};
pub struct ChangeSceneCommand {
    pub next_scene: String,
}

impl Command for ChangeSceneCommand {
    fn id(&self) -> String {
        format!("change_scene")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        context.set_current_scene(self.next_scene.as_str());
        Ok(())
    }
}
use anyhow::Result;
use crate::{Command, CommandContext, TimeManagementApi};

pub struct DebugPauseCommand;

impl Command for DebugPauseCommand{
    fn id(&self) -> String{
        "debug_pause".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>{
        context.toggle_pause();
        Ok(())
    }
}

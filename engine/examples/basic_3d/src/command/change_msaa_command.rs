use kani_volcano_engine::prelude::*;
use anyhow::{Result};

pub struct ChangeMsaaCommand{
    pub msaa: Msaa,
}

impl Command for ChangeMsaaCommand{
    fn id(&self) -> String{
        format!("change_msaa")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>{
        context.set_msaa(self.msaa);
        Ok(())
    }
}
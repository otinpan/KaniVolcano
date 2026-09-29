use kani_volcano_engine::*;
use kani_volcano_math::Transform;
use anyhow::{Result};
use crate::{MoveComponent};


#[derive(Clone, Debug)]
pub struct MoveSystem;

impl UpdateSystem for MoveSystem{
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()>{
        let delta_time=context.delta_seconds();

        for (_,transform_c,move_c) in context.query2_mut_mut::<Transform,MoveComponent>(){
            let delta=move_c.velocity*delta_time;
            transform_c.translate(delta);
        }
        Ok(())
    }
}
use anyhow::{Result, ensure};
use cgmath::{InnerSpace, Quaternion, Rad, Rotation, Rotation3};
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;

use crate::MoveRotateComponent;

#[derive(Clone, Debug)]
pub struct MoveRotateSystem;

impl UpdateSystem for MoveRotateSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let delta_time = context.delta_seconds();

        for (_, transform, move_rotate) in
            context.query2_mut_mut::<Transform, MoveRotateComponent>()
        {
            ensure!(
                move_rotate.up.magnitude2() > 0.000001,
                "MoveRotateComponent.up must not be zero"
            );

            let up = move_rotate.up.normalize();

            let sign = if move_rotate.clock_wise {
                -1.0
            } else {
                1.0
            };

            let angle = move_rotate.speed * delta_time * sign;

            let offset = transform.position - move_rotate.center;

            let rotation = Quaternion::from_axis_angle(up, Rad(angle));

            transform.position =
                move_rotate.center + rotation.rotate_vector(offset);
        }

        Ok(())
    }
}
use anyhow::{Result, anyhow};
use cgmath::vec3;
use kani_volcano_math::Transform;
use renderer_vulkan::PipelineKey;

use super::{Command, CommandContext};
use crate::Material;
use crate::primitive::PrimitiveType;
use crate::{AssetApi, InputApi, ObjectApi};

#[derive(Clone, Debug)]
pub struct SpawnPrimitiveCommand {
    pub primitive_type: PrimitiveType,
    pub pipeline_key: PipelineKey,
    pub texture_name: Option<&'static str>,
}

impl Command for SpawnPrimitiveCommand {
    fn id(&self) -> String {
        format!(
            "spawn_primitive:{:?}:{:?}:{:?}",
            self.primitive_type, self.pipeline_key, self.texture_name
        )
    }
    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        let position = mouse_position_on_spawn_plane(context);

        let texture = match self.texture_name {
            Some(name) => context.texture(name)?,
            None => context.default_texture(),
        };

        let asset_id = context
            .primitive_asset_id(
                self.primitive_type,
                self.pipeline_key.required_vertex_layout(),
            )
            .ok_or_else(|| anyhow!("not found primitive by primitive_asset_id"))?;

        context.spawn_primitive_from_mesh(
            asset_id,
            Material {
                base_color: vec3(1.0, 1.0, 1.0).extend(1.0),
                base_color_texture: self.texture_name.is_some().then_some(texture),
                pipeline_key: self.pipeline_key,
                ..Default::default()
            },
            Transform {
                position,
                ..Default::default()
            },
        )?;

        Ok(())
    }
}

fn mouse_position_on_spawn_plane(context: &CommandContext<'_>) -> cgmath::Vector3<f32> {
    let mouse = context.mouse_position();
    let window_size = context.window_size();
    let width = window_size.x.max(1.0);
    let height = window_size.y.max(1.0);
    let aspect = width / height;
    let world_height = 4.0;

    let x = mouse.x / width - 0.5;
    let y = 0.5 - mouse.y / height;

    vec3(0.0, x * world_height * aspect, y * world_height)
}

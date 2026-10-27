use anyhow::Result;
use cgmath::Vector3;
use kani_volcano_math::Transform;
use renderer_vulkan::PipelineKey;

use super::{Command, CommandContext};
use crate::{AssetApi, ObjectApi};
use crate::{Material, PrimitiveShape};

#[derive(Clone, Debug)]
pub struct CreatePrimitiveCommand {
    pub primitive_shape: PrimitiveShape,
    pub transform: Transform,
    pub color: Vector3<f32>,
    pub alpha: f32,
    pub texture: Option<&'static str>,
    pub pipeline_key: PipelineKey,
    pub auto_release: bool,
}

impl Command for CreatePrimitiveCommand {
    fn id(&self) -> String {
        format!(
            "create_primitive:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}",
            self.primitive_shape,
            self.transform,
            self.color,
            self.alpha,
            self.texture,
            self.pipeline_key
        )
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        let texture = match self.texture {
            Some(name) => context.texture(name)?,
            None => context.default_texture(),
        };

        let material = Material {
            base_color: self.color.extend(self.alpha),
            base_color_texture: self.texture.is_some().then_some(texture),
            pipeline_key: self.pipeline_key,
            ..Material::default()
        };

        context.spawn_shape_with_material(
            self.primitive_shape.clone(),
            self.transform.clone(),
            material,
            self.auto_release,
        )?;

        Ok(())
    }
}

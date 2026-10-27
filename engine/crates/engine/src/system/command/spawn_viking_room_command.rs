use anyhow::Result;
use cgmath::vec3;
use kani_volcano_math::Transform;
use renderer_vulkan::PipelineKey;

use super::{Command, CommandContext};
use crate::{Material, MeshRenderer, Rotator};

use crate::{AssetApi, EntityApi, ObjectApi};

#[derive(Clone, Debug)]
pub struct SpawnVikingRoomCommand {
    pub positions: Vec<cgmath::Vector3<f32>>,
}

impl Default for SpawnVikingRoomCommand {
    fn default() -> Self {
        Self {
            positions: vec![
                vec3(0.0, -1.25, 1.0),
                vec3(0.0, 1.25, 1.0),
                vec3(0.0, -1.25, -1.0),
                vec3(0.0, 1.25, -1.0),
            ],
        }
    }
}

impl Command for SpawnVikingRoomCommand {
    fn id(&self) -> String {
        "spawn_viking_room".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        let viking_room_mesh3d = context.model_asset_id("viking_room")?;
        let viking_room_debug_line = context.model_asset_id("viking_room_debug_line")?;
        let viking_room_lit3d = context.model_asset_id("viking_room_lit3d")?;
        let viking_texture = context.texture("viking_room")?;
        let viking_meshes = [
            viking_room_mesh3d,
            viking_room_debug_line,
            viking_room_lit3d,
        ];
        let entities = context.entities().to_vec();
        let index = entities
            .iter()
            .filter(|entity| {
                context
                    .get_component::<MeshRenderer>(**entity)
                    .is_some_and(|mesh_renderer| {
                        mesh_renderer
                            .asset_id
                            .is_some_and(|asset_id| viking_meshes.contains(&asset_id))
                    })
            })
            .count();

        let variants = [
            ("viking_room", PipelineKey::Mesh3D, 1.0),
            ("viking_room_debug_line", PipelineKey::DebugLine3D, 1.0),
            ("viking_room", PipelineKey::Transparent3D, 0.5),
            ("viking_room_lit3d", PipelineKey::Lit3D, 1.0),
        ];

        if let (Some(position), Some(variant)) =
            (self.positions.get(index).copied(), variants.get(index))
        {
            let entity = context.spawn_model(
                variant.0,
                Transform {
                    position,
                    ..Default::default()
                },
                Material {
                    base_color: vec3(1.0, 1.0, 1.0).extend(variant.2),
                    base_color_texture: Some(viking_texture),
                    pipeline_key: variant.1,
                    ..Material::default()
                },
            )?;
            context.add_component(
                entity,
                Rotator {
                    speed: vec3(20.0, 0.0, 0.0),
                },
            );
            context.set_tags(entity, ["Model", "VikingRoom", variant.0]);
        }

        Ok(())
    }
}

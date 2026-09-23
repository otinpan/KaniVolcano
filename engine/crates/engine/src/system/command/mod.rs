#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod create_primitive_command;
mod debug_monitor;
mod despawn_last_command;
mod spawn_primitive_command;
mod spawn_viking_room_command;
mod update_primitive_meshes_command;

pub use create_primitive_command::CreatePrimitiveCommand;
pub use debug_monitor::DebugMonitor;
pub use despawn_last_command::DespawnLastCommand;
pub use spawn_primitive_command::SpawnPrimitiveCommand;
pub use spawn_viking_room_command::SpawnVikingRoomCommand;
pub use update_primitive_meshes_command::UpdatePrimitiveMeshesCommand;

use anyhow::Result;
use cgmath::Vector3;

use crate::app::DEFAULT_TEXTURE;
use crate::component::{Material, PendingPrimitiveMesh, Visibility};
use crate::primitive::spawn_primitive_from_mesh;
use crate::{AssetApi, EntityApi, InputApi, ObjectApi, RenderCommandApi, SceneCommandApi, AssetLoadApi};
use crate::{
    CommandQueue, EntityId, Input, MeshAssetId, PrimitiveShape, RenderCommandQueue, Resources,
    SceneCommandQueue, World, AssetLoadCommandQueue,
};
use kani_volcano_math::Transform;
use renderer_vulkan::PipelineKey;

pub type Vec3 = Vector3<f32>;
pub type Vec2 = cgmath::Vector2<f32>;

pub struct CommandContext<'a> {
    commands: &'a mut CommandQueue,
    world: &'a mut World,
    input: &'a Input,
    resources: &'a mut Resources,
    render_commands: &'a mut RenderCommandQueue,
    asset_load_commands: &'a mut AssetLoadCommandQueue,
    scene_commands: &'a mut SceneCommandQueue,
}

impl<'a> CommandContext<'a> {
    pub(crate) fn new(
        commands: &'a mut CommandQueue,
        world: &'a mut World,
        input: &'a Input,
        resources: &'a mut Resources,
        render_commands: &'a mut RenderCommandQueue,
        asset_load_commands: &'a mut AssetLoadCommandQueue,
        scene_commands: &'a mut SceneCommandQueue,
    ) -> Self {
        Self {
            commands,
            world,
            input,
            resources,
            render_commands,
            asset_load_commands,
            scene_commands,
        }
    }

    // resources /////////////////////////////////

    // create new primitive entity and new mesh
    // entity is create here, but mesh is created in RenderSystem using VulkanRenderer.
    // the frame this called do not render new primitive
    pub(crate) fn enqueue_spawn_shape(
        &mut self,
        shape: PrimitiveShape,
        transform: Transform,
        material: Material,
        auto_release: bool,
    ) -> Result<EntityId> {
        let entity = self.spawn();

        self.add_component(entity, transform);
        self.add_component(entity, Visibility::default());
        self.add_component(
            entity,
            PendingPrimitiveMesh {
                shape: shape.clone(),
                material,
                auto_release,
            },
        );

        self.render_commands.create_primitive_mesh(entity);

        Ok(entity)
    }
}

pub trait Command {
    fn id(&self) -> String;
    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>;
}

impl ObjectApi for CommandContext<'_> {
    fn spawn_primitive_from_mesh(
        &mut self,
        asset_id: MeshAssetId,
        material: Material,
        transform: Transform,
    ) -> Result<EntityId> {
        spawn_primitive_from_mesh(self.world, self.resources, asset_id, material, transform)
    }

    fn primitive_material(
        &self,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<Material> {
        let use_texture = texture.is_some();
        let texture = match texture {
            Some(texture_name) => self.texture(texture_name)?,
            None => DEFAULT_TEXTURE,
        };

        Ok(Material {
            color,
            alpha,
            use_texture,
            texture,
            pipeline_key,
        })
    }

    fn spawn_shape_with_material(
        &mut self,
        shape: PrimitiveShape,
        transform: Transform,
        material: Material,
        auto_release: bool,
    ) -> Result<EntityId> {
        self.enqueue_spawn_shape(shape, transform, material, auto_release)
    }
}

impl EntityApi for CommandContext<'_> {
    fn world(&self) -> &World {
        &self.world
    }
    fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    fn render_commands(&self) -> &RenderCommandQueue {
        &self.render_commands
    }
    fn render_commands_mut(&mut self) -> &mut RenderCommandQueue {
        &mut self.render_commands
    }

    fn resources(&self) -> &Resources {
        &self.resources
    }
    fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }
}

impl AssetApi for CommandContext<'_> {
    fn resources(&self) -> &Resources {
        &self.resources
    }

    fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }
}

impl InputApi for CommandContext<'_> {
    fn input(&self) -> &Input {
        &self.input
    }
}

impl RenderCommandApi for CommandContext<'_> {
    fn render_commands_mut(&mut self) -> &mut RenderCommandQueue {
        &mut self.render_commands
    }
}

impl SceneCommandApi for CommandContext<'_> {
    fn scene_commands_mut(&mut self) -> &mut SceneCommandQueue {
        &mut self.scene_commands
    }
}

impl AssetLoadApi for CommandContext<'_>{
    fn asset_load_commands_mut(&mut self) -> &mut AssetLoadCommandQueue{
        &mut self.asset_load_commands
    }
}

#[derive(Clone, Debug)]
pub struct CommandSystem;

impl CommandSystem {
    pub fn update(&self, context: &mut CommandContext<'_>) -> Result<()> {
        let commands = context.commands.drain(..).collect::<Vec<_>>();

        for command in commands {
            command.execute(context)?;
        }

        Ok(())
    }
}

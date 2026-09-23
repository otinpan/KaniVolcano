#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]
use anyhow::Result;
use cgmath::{Vector2, Vector3};

mod camera_system;
mod rotator_system;

pub use super::render_command::RenderCommandQueue;
pub use super::asset_load_command::AssetLoadCommandQueue;
pub use super::scene_command::SceneCommandQueue;
use crate::app::DEFAULT_TEXTURE;
use crate::component::{Material, Visibility};
use crate::primitive::spawn_primitive_from_mesh;
use crate::{
    EntityId, Input, MeshAssetId, PendingPrimitiveMesh, PrimitiveShape, Resources, Time, World,
};
use kani_volcano_math::Transform;
use renderer_vulkan::PipelineKey;

use crate::{AssetApi, EntityApi, InputApi, ObjectApi, RenderCommandApi, SceneCommandApi, TimeApi, AssetLoadApi};
pub use camera_system::CameraSystem;
pub use rotator_system::RotatorSystem;

type Vec2 = Vector2<f32>;
type Vec3 = Vector3<f32>;

pub struct UpdateContext<'a> {
    world: &'a mut World,
    input: &'a Input,
    time: &'a Time,
    resources: &'a mut Resources,
    render_commands: &'a mut RenderCommandQueue,
    asset_load_commands: &'a mut AssetLoadCommandQueue,
    scene_commands: &'a mut SceneCommandQueue,
}

impl<'a> UpdateContext<'a> {
    pub(crate) fn new(
        world: &'a mut World,
        input: &'a Input,
        time: &'a Time,
        resources: &'a mut Resources,
        render_commands: &'a mut RenderCommandQueue,
        asset_load_commands: &'a mut AssetLoadCommandQueue,
        scene_commands: &'a mut SceneCommandQueue,
    ) -> Self {
        Self {
            world,
            input,
            time,
            resources,
            render_commands,
            asset_load_commands,
            scene_commands,
        }
    }

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

impl EntityApi for UpdateContext<'_> {
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

impl ObjectApi for UpdateContext<'_> {
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

impl AssetApi for UpdateContext<'_> {
    fn resources(&self) -> &Resources {
        &self.resources
    }

    fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }
}

impl InputApi for UpdateContext<'_> {
    fn input(&self) -> &Input {
        &self.input
    }
}

impl TimeApi for UpdateContext<'_> {
    fn time(&self) -> &Time {
        &self.time
    }
}

impl RenderCommandApi for UpdateContext<'_> {
    fn render_commands_mut(&mut self) -> &mut RenderCommandQueue {
        &mut self.render_commands
    }
}

impl SceneCommandApi for UpdateContext<'_> {
    fn scene_commands_mut(&mut self) -> &mut SceneCommandQueue {
        &mut self.scene_commands
    }
}

impl AssetLoadApi for UpdateContext<'_>{
    fn asset_load_commands_mut(&mut self) -> &mut AssetLoadCommandQueue{
        &mut self.asset_load_commands
    }
}

pub struct ScheduledUpdateSystem {
    pub name: String,
    pub system: Box<dyn UpdateSystem>,
    pub enabled: bool,
}
pub trait UpdateSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()>;
}

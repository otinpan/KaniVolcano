use crate::app::DEFAULT_TEXTURE;
use crate::component::Visibility;
use crate::primitive::{PrimitiveShape, spawn_primitive_from_mesh};
use crate::system::{
    AssetApi, Command, EntityApi, InputTrigger, ObjectApi, RenderCommandApi, UpdateContext,
    UpdateSystem,
};
use crate::{
    EntityId, Input, Material, MeshAssetId, PendingPrimitiveMesh, RenderCommandQueue, Resources,
    SceneId, SceneOwned, Scheduler, Time, World,
};
use cgmath::Vector3;
use renderer_vulkan::PipelineKey;

use anyhow::Result;
use kani_volcano_math::Transform;

type Vec3 = Vector3<f32>;

pub trait Scene {
    fn name(&self) -> String;
    fn on_enter_scene(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        self.on_enter(context)?;
        Ok(())
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()>;

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()>;

    fn on_exit_scene(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.despawn_scene_owned_entities();
        self.on_exit(context)?;
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()>;

}

pub struct SceneContext<'a> {
    scene_id: SceneId,
    world: &'a mut World,
    input: &'a Input,
    time: &'a Time,
    // resource
    resources: &'a mut Resources,
    // system
    scheduler: &'a mut Scheduler,
}

impl<'a> SceneContext<'a> {
    pub(crate) fn new(
        scene_id: SceneId,
        world: &'a mut World,
        input: &'a Input,
        time: &'a Time,
        resources: &'a mut Resources,
        scheduler: &'a mut Scheduler,
    ) -> Self {
        Self {
            scene_id,
            world,
            input,
            time,
            resources,
            scheduler,
        }
    }

    pub fn scene_id(&self) -> SceneId {
        self.scene_id
    }

    pub fn despawn_scene_owned_entities(&mut self) -> usize {
        let scene_id = self.scene_id;
        let entities = self
            .query1::<SceneOwned>()
            .filter_map(|(entity, scene_owned)| {
                (scene_owned.scene_id == scene_id).then_some(entity)
            })
            .collect::<Vec<_>>();

        entities
            .into_iter()
            .filter(|entity| self.despawn(*entity))
            .count()
    }

    // delete_scene_owned let entity be global entity.
    // not released when scene is released
    fn delete_scene_owned(&mut self, entity: EntityId) {
        self.remove_component::<SceneOwned>(entity);
    }
    // bind Command with (key,trigger)
    pub fn bind_input_command<C>(
        &mut self,
        key: winit::keyboard::KeyCode,
        trigger: InputTrigger,
        command: C,
    ) where
        C: Command + 'static,
    {
        self.scheduler.bind_key(key, trigger, command);
    }

    // insert UpdateSystem to Scheduler
    pub fn add_update_system<S>(&mut self, name: &str, system: S)
    where
        S: UpdateSystem + 'static,
    {
        self.scheduler.add_update_system(name, system);
    }
    pub fn add_fixed_update_system<S>(&mut self, name: &str, system: S)
    where 
        S: UpdateSystem + 'static,
    {
        self.scheduler.add_fixed_update_system(name,system);
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

        self.scheduler.render_commands.create_primitive_mesh(entity);

        Ok(entity)
    }
}

impl EntityApi for SceneContext<'_> {
    // override EntityApi spawn
    fn spawn(&mut self) -> EntityId {
        let entity = self.world.spawn();
        self.world.add_component(
            entity,
            SceneOwned {
                scene_id: self.scene_id,
            },
        );

        entity
    }
    fn world(&self) -> &World {
        &self.world
    }
    fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    fn resources(&self) -> &Resources {
        &self.resources
    }
    fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }

    fn render_commands(&self) -> &RenderCommandQueue {
        &self.scheduler.render_commands
    }
    fn render_commands_mut(&mut self) -> &mut RenderCommandQueue {
        &mut self.scheduler.render_commands
    }
}

impl AssetApi for SceneContext<'_> {
    fn resources(&self) -> &Resources {
        &self.resources
    }
    fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }
}

impl ObjectApi for SceneContext<'_> {
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

impl RenderCommandApi for SceneContext<'_> {
    fn render_commands_mut(&mut self) -> &mut RenderCommandQueue {
        &mut self.scheduler.render_commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct MockScene {
        global_entity: Option<EntityId>,
    }

    impl Scene for MockScene {
        fn name(&self) -> String {
            "MockScene".to_string()
        }

        fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
            let entity0 = context.spawn();
            let entity1 = context.spawn();
            let entity2 = context.spawn();
            let entity3 = context.spawn();
            context.delete_scene_owned(entity3);

            assert!(context.has_component::<SceneOwned>(entity0));
            assert!(context.has_component::<SceneOwned>(entity1));
            assert!(context.has_component::<SceneOwned>(entity2));
            assert!(!context.has_component::<SceneOwned>(entity3));

            self.global_entity = Some(entity3);
            Ok(())
        }

        fn update(&mut self, _context: &mut UpdateContext<'_>) -> Result<()> {
            Ok(())
        }

        fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
            let count = context.despawn_scene_owned_entities();
            assert_eq!(count, 3);

            let global_entity = self.global_entity.unwrap();
            assert!(context.is_entity_registered(global_entity));
            assert!(!context.has_component::<SceneOwned>(global_entity));
            Ok(())
        }
    }

    #[test]
    fn scene_context_despawns_only_current_scene_owned_entities() {
        let scene_id = SceneId(1);
        let other_scene_id = SceneId(2);
        let mut world = World::default();
        let other_scene_entity = world.spawn();
        world.add_component(
            other_scene_entity,
            SceneOwned {
                scene_id: other_scene_id,
            },
        );

        let input = Input::default();
        let time = Time::default();
        let mut resources = Resources::default();
        let mut scheduler = Scheduler::default();

        let mut scene = MockScene {
            global_entity: None,
        };

        {
            let mut context = SceneContext {
                scene_id,
                world: &mut world,
                input: &input,
                time: &time,
                resources: &mut resources,
                scheduler: &mut scheduler,
            };

            scene.on_enter(&mut context).unwrap();
            assert_eq!(context.entity_count(), 5);
            scene.on_exit(&mut context).unwrap();
        }

        assert_eq!(world.entity_count(), 2);
        assert!(world.contains(other_scene_entity));
        assert_eq!(
            world
                .get_component::<SceneOwned>(other_scene_entity)
                .unwrap()
                .scene_id,
            other_scene_id
        );

        let global_entity = scene.global_entity.unwrap();
        assert!(world.contains(global_entity));
        assert!(!world.has_component::<SceneOwned>(global_entity));
    }
}

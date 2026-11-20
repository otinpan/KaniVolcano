use anyhow::{ensure, Result};
use kani_volcano_math::Transform;
use super::{UpdateContext, UpdateSystem};
use crate::{EntityApi, EntityId, GlobalTransform, ParentComponent, World};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct TransformSystem{

}

impl UpdateSystem for TransformSystem{
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()>{
        self.update_world(context.world_mut())
    }
}

impl TransformSystem {
    pub(crate) fn update_world(&mut self, world: &mut World) -> Result<()> {
        let local: HashMap<_, _> = world.query1::<Transform>()
            .map(|(entity, transform)| (entity, transform.matrix())).collect();
        let mut children = HashMap::<EntityId, Vec<EntityId>>::new();
        let mut pending = Vec::new();
        for (&entity, &matrix) in &local {
            if let Some(parent) = world.get_component::<ParentComponent>(entity) {
                ensure!(local.contains_key(&parent.parent),
                    "Parent {:?} of {:?} is missing or has no Transform", parent.parent, entity);
                children.entry(parent.parent).or_default().push(entity);
            } else {
                pending.push((entity, matrix));
            }
        }
        // Traverse parent-first. Each local matrix is composed only once.
        let mut computed = Vec::with_capacity(local.len());
        while let Some((entity, matrix)) = pending.pop() {
            if let Some(children) = children.remove(&entity) {
                for child in children {
                    pending.push((child, matrix * local[&child]));
                }
            }
            computed.push((entity, matrix));
        }
        ensure!(computed.len() == local.len(), "Cycle in Transform parent hierarchy");

        // Commit only after the entire hierarchy has been validated.
        let stale: Vec<_> = world.query1::<GlobalTransform>()
            .filter_map(|(entity, _)| (!local.contains_key(&entity)).then_some(entity)).collect();
        for entity in stale {
            world.remove_component::<GlobalTransform>(entity);
        }
        for (entity, matrix) in computed {
            if let Some(global) = world.get_component_mut::<GlobalTransform>(entity) {
                global.matrix = matrix;
            } else {
                world.add_component(entity, GlobalTransform { matrix });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::vec3;

    #[test]
    fn creates_and_updates_world_matrices_without_changing_local_transforms() {
        let mut world = World::default();
        // Child created first to verify independence from entity iteration order.
        let child = world.spawn();
        let parent = world.spawn();
        let standalone = world.spawn();
        let parent_transform = Transform {
            position: vec3(2.0, 3.0, 4.0), rotation: vec3(0.0, 0.0, 90.0),
            scale: vec3(2.0, 3.0, 1.0),
        };
        let child_transform = Transform {
            position: vec3(1.0, 0.0, 0.0), rotation: vec3(0.0, 30.0, 0.0),
            ..Default::default()
        };
        world.add_component(parent, parent_transform.clone());
        world.add_component(child, child_transform.clone());
        world.add_component(child, ParentComponent::new(parent));
        world.add_component(standalone, Transform::default());
        let mut system = TransformSystem::default();
        system.update_world(&mut world).unwrap();
        assert_eq!(world.get_component::<GlobalTransform>(child).unwrap().matrix,
            parent_transform.matrix() * child_transform.matrix());
        assert_eq!(world.get_component::<Transform>(child).unwrap().matrix(), child_transform.matrix());
        assert_eq!(world.get_component::<GlobalTransform>(standalone).unwrap().matrix, Transform::default().matrix());
        world.get_component_mut::<Transform>(parent).unwrap().position.x += 5.0;
        system.update_world(&mut world).unwrap();
        assert_eq!(world.get_component::<GlobalTransform>(child).unwrap().matrix,
            world.get_component::<Transform>(parent).unwrap().matrix() * child_transform.matrix());
        world.remove_component::<ParentComponent>(child);
        world.remove_component::<Transform>(standalone);
        system.update_world(&mut world).unwrap();
        assert_eq!(world.get_component::<GlobalTransform>(child).unwrap().matrix, child_transform.matrix());
        assert!(world.get_component::<GlobalTransform>(standalone).is_none());
    }

    #[test]
    fn rejects_missing_parents_and_cycles_without_partial_updates() {
        let mut world = World::default();
        let a = world.spawn();
        let b = world.spawn();
        world.add_component(a, Transform::default());
        world.add_component(b, Transform::default());
        let mut system = TransformSystem::default();
        system.update_world(&mut world).unwrap();
        let original = world.get_component::<GlobalTransform>(a).unwrap().matrix;
        world.get_component_mut::<Transform>(a).unwrap().position.x = 10.0;
        world.add_component(a, ParentComponent::new(b));
        world.add_component(b, ParentComponent::new(a));
        assert!(system.update_world(&mut world).is_err());
        assert_eq!(world.get_component::<GlobalTransform>(a).unwrap().matrix, original);
        world.add_component(b, ParentComponent::new(EntityId(999)));
        assert!(system.update_world(&mut world).is_err());
        world.remove_component::<ParentComponent>(b);
        world.remove_component::<Transform>(b);
        assert!(system.update_world(&mut world).is_err());
    }
}

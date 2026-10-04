use std::any::{Any, TypeId};
use std::collections::HashMap;

use super::{ComponentPool, EntityId};

use crate::component::Component;

pub type Vec3 = cgmath::Vector3<f32>;

trait ErasedComponentPool {
    fn remove_entity(&mut self, entity: EntityId);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> ErasedComponentPool for ComponentPool<T> {
    fn remove_entity(&mut self, entity: EntityId) {
        self.remove(entity);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct Registry {
    next_entity_id: usize,
    entities: Vec<EntityId>,

    // component
    component_pools: HashMap<TypeId, Box<dyn ErasedComponentPool>>,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("next_entity_id", &self.next_entity_id)
            .field("entities", &self.entities)
            .field("component_pool_count", &self.component_pools.len())
            .finish()
    }
}

impl Registry {
    pub fn create(&mut self) -> EntityId {
        let entity = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.push(entity);
        entity
    }

    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }

    pub fn contains(&self, entity: EntityId) -> bool {
        self.entities.contains(&entity)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    // if there is not ComponentPool<T>, create it and get pool
    fn get_or_create_pool<T: Component>(&mut self) -> &mut ComponentPool<T> {
        let type_id = TypeId::of::<T>();

        self.component_pools
            .entry(type_id)
            .or_insert_with(|| Box::new(ComponentPool::<T>::new()));

        self.component_pools
            .get_mut(&type_id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<ComponentPool<T>>()
            .unwrap()
    }

    // Component access ////////////////////////////////////////////
    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) -> bool {
        if !self.contains(entity) {
            return false;
        }

        self.get_or_create_pool::<T>().insert(entity, component);
        true
    }

    // remove component from entity, but not remove entity
    pub fn remove_component<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        self.get_pool_mut::<T>()?.remove(entity)
    }

    pub fn despawn(&mut self, entity: EntityId) -> bool {
        if !self.contains(entity) {
            return false;
        }

        for pool in self.component_pools.values_mut() {
            pool.remove_entity(entity);
        }

        self.entities.retain(|id| *id != entity);
        true
    }

    pub fn get_pool<T: Component>(&self) -> Option<&ComponentPool<T>> {
        self.component_pools
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<ComponentPool<T>>()
    }

    pub fn get_pool_mut<T: Component>(&mut self) -> Option<&mut ComponentPool<T>> {
        self.component_pools
            .get_mut(&TypeId::of::<T>())?
            .as_any_mut()
            .downcast_mut::<ComponentPool<T>>()
    }

    pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        self.get_pool::<T>()?.get(entity)
    }

    pub fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.get_pool_mut::<T>()?.get_mut(entity)
    }

    pub fn has_component<T: Component>(&self, entity: EntityId) -> bool {
        self.get_pool::<T>()
            .is_some_and(|pool| pool.contains(entity))
    }

    // query ////////
    pub fn query1<A>(&self) -> Box<dyn Iterator<Item = (EntityId, &A)> + '_>
    where
        A: Component,
    {
        let Some(a_pool) = self.get_pool::<A>() else {
            return Box::new(std::iter::empty());
        };

        Box::new(a_pool.iter())
    }

    pub fn query1_mut<A>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A)> + '_>
    where
        A: Component,
    {
        let Some(a_pool) = self.get_pool_mut::<A>() else {
            return Box::new(std::iter::empty());
        };

        Box::new(a_pool.iter_mut())
    }

    pub fn query2<A, B>(&self) -> Box<dyn Iterator<Item = (EntityId, &A, &B)> + '_>
    where
        A: Component,
        B: Component,
    {
        let Some(a_pool) = self.get_pool::<A>() else {
            return Box::new(std::iter::empty());
        };
        let Some(b_pool) = self.get_pool::<B>() else {
            return Box::new(std::iter::empty());
        };

        Box::new(a_pool.iter().filter_map(move |(entity, a)| {
            let b = b_pool.get(entity)?;
            Some((entity, a, b))
        }))
    }

    // UNSAFE: if not using unsafe, returned entity can not use mutable and imutable
    // because, entity will use registry as mutable and immutable!
    // Returns entities with A and B, borrowing A mutably and B immutably.
    pub fn query2_mut<A, B>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A, &B)> + '_>
    where
        A: Component,
        B: Component,
    {
        assert_ne!(
            TypeId::of::<A>(),
            TypeId::of::<B>(),
            "query2_mut cannot borrow the same component type as both mutable and immutable"
        );

        let registry = self as *mut Registry;

        let Some(a_pool) = (unsafe {
            (&mut *registry)
                .get_pool_mut::<A>()
                .map(|pool| pool as *mut ComponentPool<A>)
        }) else {
            return Box::new(std::iter::empty());
        };
        let Some(b_pool) = (unsafe {
            (&*registry)
                .get_pool::<B>()
                .map(|pool| pool as *const ComponentPool<B>)
        }) else {
            return Box::new(std::iter::empty());
        };

        Box::new(unsafe {
            (*a_pool).iter_mut().filter_map(move |(entity, a)| {
                let b = (*b_pool).get(entity)?;
                Some((entity, a, b))
            })
        })
    }
    pub fn query2_mut_mut<A, B>(
        &mut self,
    ) -> Box<dyn Iterator<Item = (EntityId, &mut A, &mut B)> + '_>
    where
        A: Component,
        B: Component,
    {
        assert_ne!(
            TypeId::of::<A>(),
            TypeId::of::<B>(),
            "query2_mut_mut cannot borrow the same component type mutably twice"
        );

        let registry = self as *mut Registry;

        let Some(a_pool) = (unsafe {
            (&mut *registry)
                .get_pool_mut::<A>()
                .map(|pool| pool as *mut ComponentPool<A>)
        }) else {
            return Box::new(std::iter::empty());
        };
        let Some(b_pool) = (unsafe {
            (&mut *registry)
                .get_pool_mut::<B>()
                .map(|pool| pool as *mut ComponentPool<B>)
        }) else {
            return Box::new(std::iter::empty());
        };

        Box::new(unsafe {
            (*a_pool).iter_mut().filter_map(move |(entity, a)| {
                let b = (*b_pool).get_mut(entity)?;
                Some((entity, a, b))
            })
        })
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            next_entity_id: 0,
            entities: Vec::new(),

            component_pools: HashMap::new(),
        }
    }
}

// test ///////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Material, MeshRenderer, Rotator, Visibility};
    use cgmath::vec3;
    use kani_volcano_math::Transform;
    use renderer_vulkan::{MeshHandle, VertexLayout};

    fn mesh_handle(index: usize) -> MeshHandle {
        MeshHandle::new(index, VertexLayout::Mesh3D)
    }

    fn spawn_renderable(
        registry: &mut Registry,
        mesh: MeshHandle,
        transform: Transform,
    ) -> EntityId {
        let entity = registry.create();
        registry.add_component(entity, transform);
        registry.add_component(
            entity,
            MeshRenderer {
                mesh,
                asset_id: None,
                material: Material::default(),
            },
        );
        registry.add_component(
            entity,
            Rotator {
                speed: vec3(20.0, 0.0, 0.0),
            },
        );
        registry.add_component(entity, Visibility::default());

        entity
    }

    #[test]
    fn create_adds_entity_and_returns_unique_entity_ids() {
        let mut registry = Registry::default();
        let mesh = mesh_handle(0);

        let first = spawn_renderable(&mut registry, mesh, Transform::default());
        let second = spawn_renderable(&mut registry, mesh, Transform::default());

        assert_eq!(registry.entity_count(), 2);
        assert_ne!(first, second);
        assert_eq!(first, EntityId(0));
        assert_eq!(second, EntityId(1));
    }

    #[test]
    fn despawn_removes_entity_and_components() {
        let mut registry = Registry::default();
        let first = spawn_renderable(&mut registry, mesh_handle(0), Transform::default());
        let second = spawn_renderable(&mut registry, mesh_handle(1), Transform::default());

        assert!(registry.despawn(first));

        assert_eq!(registry.entity_count(), 1);
        assert!(!registry.contains(first));
        assert!(registry.contains(second));
        assert!(registry.get_component::<Transform>(first).is_none());
        assert!(registry.get_component::<MeshRenderer>(first).is_none());
    }

    #[test]
    fn despawn_unknown_entity_returns_false() {
        let mut registry = Registry::default();

        assert!(!registry.despawn(EntityId(999)));
        assert_eq!(registry.entity_count(), 0);
    }

    #[test]
    fn component_access_can_add_get_and_remove_components() {
        let mut registry = Registry::default();
        let entity = registry.create();
        registry.add_component(entity, Transform::default());

        assert!(registry.add_component(
            entity,
            Rotator {
                speed: vec3(1.0, 2.0, 3.0),
            }
        ));
        assert!(registry.has_component::<Rotator>(entity));
        assert_eq!(
            registry.get_component::<Rotator>(entity).unwrap().speed.x,
            1.0
        );
        assert!(registry.remove_component::<Rotator>(entity).is_some());
        assert!(!registry.has_component::<Rotator>(entity));
    }

    #[test]
    fn query1_iterates_entities_with_component() {
        let mut registry = Registry::default();
        let matching = registry.create();
        registry.add_component(matching, Transform::default());
        let other = registry.create();
        registry.add_component(
            other,
            Rotator {
                speed: vec3(1.0, 2.0, 3.0),
            },
        );

        let entities = registry
            .query1::<Transform>()
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();

        assert_eq!(entities, vec![matching]);
    }

    #[test]
    fn query1_mut_iterates_entities_with_component() {
        let mut registry = Registry::default();
        let matching = registry.create();
        registry.add_component(matching, Transform::default());

        for (_, transform) in registry.query1_mut::<Transform>() {
            transform.rotate(vec3(1.0, 2.0, 3.0));
        }

        assert_eq!(
            registry
                .get_component::<Transform>(matching)
                .unwrap()
                .rotation,
            vec3(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn query2_iterates_entities_with_both_components() {
        let mut registry = Registry::default();
        let matching = spawn_renderable(&mut registry, mesh_handle(0), Transform::default());
        let entity = registry.create();
        registry.add_component(entity, Transform::default());

        let entities = registry
            .query2::<Transform, MeshRenderer>()
            .map(|(entity, _, _)| entity)
            .collect::<Vec<_>>();

        assert_eq!(entities, vec![matching]);
    }

    #[test]
    fn query2_mut_iterates_entities_with_both_components() {
        let mut registry = Registry::default();
        let matching = spawn_renderable(&mut registry, mesh_handle(0), Transform::default());
        let transform_only = registry.create();
        registry.add_component(transform_only, Transform::default());

        let entities = registry
            .query2_mut::<Transform, Rotator>()
            .map(|(entity, transform, rotator)| {
                transform.rotate(rotator.speed);
                entity
            })
            .collect::<Vec<_>>();

        assert_eq!(entities, vec![matching]);
        assert_eq!(
            registry
                .get_component::<Transform>(matching)
                .unwrap()
                .rotation,
            vec3(20.0, 0.0, 0.0)
        );
        assert_eq!(
            registry
                .get_component::<Transform>(transform_only)
                .unwrap()
                .rotation,
            vec3(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn query2_mut_mut_can_split_distinct_entities_across_threads() {
        let mut registry = Registry::default();
        let entities: Vec<_> = (0..64)
            .map(|_| {
                let entity = registry.create();
                registry.add_component(entity, Transform::default());
                registry.add_component(
                    entity,
                    Rotator {
                        speed: vec3(1.0, 0.0, 0.0),
                    },
                );
                entity
            })
            .collect();

        let mut entries: Vec<_> = registry.query2_mut_mut::<Transform, Rotator>().collect();
        assert_eq!(entries.len(), entities.len());

        std::thread::scope(|scope| {
            for chunk in entries.chunks_mut(16) {
                scope.spawn(move || {
                    for (_, transform, rotator) in chunk {
                        transform.rotate(rotator.speed);
                    }
                });
            }
        });
        drop(entries);

        for entity in entities {
            assert_eq!(
                registry.get_component::<Transform>(entity).unwrap().rotation,
                vec3(1.0, 0.0, 0.0)
            );
        }
    }

    #[test]
    #[should_panic(expected = "query2_mut_mut cannot borrow the same component type mutably twice")]
    fn query2_mut_mut_rejects_the_same_component_type() {
        let mut registry = Registry::default();
        let _ = registry.query2_mut_mut::<Transform, Transform>();
    }
}

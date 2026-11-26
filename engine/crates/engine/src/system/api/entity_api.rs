use crate::{
    Component, ComponentPool, EntityId, MeshRenderer, RenderCommandQueue, Resources, World,
    MeshAssetId, GltfModelRoot, ParentComponent, PendingPrimitiveMesh,
};
use std::collections::{HashMap, HashSet};

pub trait EntityApi {
    fn world(&self) -> &World;
    fn world_mut(&mut self) -> &mut World;

    fn render_commands(&self) -> &RenderCommandQueue;
    fn render_commands_mut(&mut self) -> &mut RenderCommandQueue;

    fn resources(&self) -> &Resources;
    fn resources_mut(&mut self) -> &mut Resources;

    fn spawn(&mut self) -> EntityId {
        self.world_mut().spawn()
    }

    fn despawn(&mut self, entity: EntityId) -> bool {
        if !self.world_mut().contains(entity) {
            return false;
        }

        let mut children = HashMap::<EntityId, Vec<EntityId>>::new();
        for (child, parent) in self.world().query1::<ParentComponent>() {
            children.entry(parent.parent).or_default().push(child);
        }
        let mut stack = vec![entity];
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                order.push(current);
                if let Some(children) = children.remove(&current) {
                    stack.extend(children);
                }
            }
        }
        // Descendants release their mesh references before the model's ownership reference.
        for current in order.into_iter().rev() {
            let materials = [
                self.world().get_component::<MeshRenderer>(current).map(|renderer| renderer.material),
                self.world().get_component::<PendingPrimitiveMesh>(current).map(|pending| pending.material),
            ];
            for material in materials.into_iter().flatten() {
                let handles = self.resources_mut().release_material_textures(&material);
                self.render_commands_mut().defer_texture_destruction(handles);
            }
            let mesh_id = self.world().get_component::<MeshRenderer>(current)
                .and_then(|renderer| renderer.asset_id);
            if let Some(id) = mesh_id {
                if let Some(handle) = self.resources_mut().release_mesh(id) {
                    self.render_commands_mut().destroy_mesh(handle);
                }
            }
            let model_id = self.world().get_component::<GltfModelRoot>(current)
                .map(|root| root.model_id);
            // decrement model reference counter and children's mesh reference counter
            if let Some(id) = model_id {
                for handle in self.resources_mut().release_gltf_model(id) {
                    // if chidlren's mesh reference counter is zero, release mesh
                    self.render_commands_mut().destroy_mesh(handle);
                }
            }
            self.world_mut().despawn(current);
        }
        true
    }

    fn despawn_last(&mut self) -> bool {
        let Some(entity) = self.entities().last().copied() else {
            return false;
        };

        self.despawn(entity)
    }

    fn entities(&self) -> &[EntityId] {
        self.world().entities()
    }

    fn is_entity_registered(&self, entity: EntityId) -> bool {
        self.world().contains(entity)
    }

    fn entity_count(&self) -> usize {
        self.world().entity_count()
    }

    fn add_component<T: Component>(&mut self, entity: EntityId, component: T) -> bool {
        self.world_mut().add_component::<T>(entity, component)
    }

    fn remove_component<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        self.world_mut().remove_component::<T>(entity)
    }

    fn get_component_pool<T: Component>(&self) -> Option<&ComponentPool<T>> {
        self.world().get_pool::<T>()
    }

    fn get_component_pool_mut<T: Component>(&mut self) -> Option<&mut ComponentPool<T>> {
        self.world_mut().get_pool_mut::<T>()
    }

    fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        self.world().get_component::<T>(entity)
    }

    fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.world_mut().get_component_mut::<T>(entity)
    }

    fn has_component<T: Component>(&self, entity: EntityId) -> bool {
        self.world().has_component::<T>(entity)
    }

    fn query1<A>(&self) -> Box<dyn Iterator<Item = (EntityId, &A)> + '_>
    where
        A: Component,
    {
        self.world().query1::<A>()
    }

    fn query1_mut<A>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A)> + '_>
    where
        A: Component,
    {
        self.world_mut().query1_mut::<A>()
    }

    fn query2<A, B>(&self) -> Box<dyn Iterator<Item = (EntityId, &A, &B)> + '_>
    where
        A: Component,
        B: Component,
    {
        self.world().query2::<A, B>()
    }

    fn query2_mut<A, B>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A, &B)> + '_>
    where
        A: Component,
        B: Component,
    {
        self.world_mut().query2_mut::<A, B>()
    }

    fn query2_mut_mut<A, B>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A, &mut B)> + '_>
    where
        A: Component,
        B: Component,
    {
        self.world_mut().query2_mut_mut::<A, B>()
    }

    fn find_entity_by_name(&self, name: &str) -> Option<EntityId> {
        self.world().find_by_name(name)
    }

    fn set_name(&mut self, entity: EntityId, name: &str) -> bool {
        self.world_mut().set_name(entity, name)
    }

    fn remove_name(&mut self, entity: EntityId) -> bool {
        self.world_mut().remove_name(entity)
    }

    fn set_tags<const N: usize>(&mut self, entity: EntityId, tags: [&str; N]) -> bool {
        self.world_mut().set_tags(entity, tags)
    }

    fn remove_tags(&mut self, entity: EntityId) -> bool {
        self.world_mut().remove_tags(entity)
    }

    fn remove_tag(&mut self, entity: EntityId, tag: &str) -> bool {
        self.world_mut().remove_tag(entity, tag)
    }
    fn get_entities_by_tag(&self, tag: &str) -> Vec<EntityId> {
        self.world().find_by_tag(tag)
    }

    fn get_all_named_entities(&self) -> Vec<(String, EntityId)> {
        self.world().get_all_named_entities()
    }

    fn get_all_taged_entities(&self) -> Vec<(String, EntityId)> {
        self.world().get_all_taged_entities()
    }


    fn mesh_asset_id(&self, entity: EntityId) -> Option<MeshAssetId> {
        self.get_component::<MeshRenderer>(entity)
            .and_then(|renderer| renderer.asset_id)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Material;
    use renderer_vulkan::{MeshHandle, VertexLayout};

    #[derive(Default)]
    struct TestContext {
        world: World,
        resources: Resources,
        render_commands: RenderCommandQueue,
    }

    impl EntityApi for TestContext {
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
            &self.render_commands
        }

        fn render_commands_mut(&mut self) -> &mut RenderCommandQueue {
            &mut self.render_commands
        }
    }

    fn mesh_renderer(asset_id: Option<MeshAssetId>) -> MeshRenderer {
        MeshRenderer {
            mesh: MeshHandle::new(0, VertexLayout::Mesh3D),
            asset_id,
            material: Material::default(),
        }
    }

    #[test]
    fn despawn_releases_renderer_and_pending_texture_ownership() {
        for auto_release in [false, true] {
            let mut context = TestContext::default();
            let handle = renderer_vulkan::TextureHandle(1);
            let texture = context.resources.register_texture("shared", handle, auto_release).unwrap();
            let material = Material { base_color_texture: Some(texture),
                emissive_texture: Some(texture), ..Material::default() };
            let root = context.spawn();
            let child = context.spawn();
            let unrelated = context.spawn();
            for _ in 0..3 {
                context.resources.retain_material_textures(&material).unwrap();
            }
            context.add_component(root, MeshRenderer { material, ..mesh_renderer(None) });
            context.add_component(unrelated, MeshRenderer { material, ..mesh_renderer(None) });
            context.add_component(child, ParentComponent::new(root));
            context.add_component(child, PendingPrimitiveMesh {
                shape: crate::PrimitiveShape::Sphere { radius: 1.0, rings: 8, segments: 8,
                    color: material.base_color.truncate() },
                material,
                auto_release: true,
            });
            assert!(context.despawn(root));
            assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 1);
            assert!(context.render_commands.pending_texture_destroys().is_empty());
            assert!(!context.despawn(root));
            assert!(context.despawn(unrelated));
            if auto_release {
                assert!(context.resources.get_texture_handle(texture).is_none());
                assert_eq!(context.render_commands.pending_texture_destroys(), &[handle]);
            } else {
                assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 0);
                assert!(context.render_commands.pending_texture_destroys().is_empty());
            }
            assert!(!context.despawn(unrelated));
            // Draining ordinary commands must not discard pending GPU cleanup.
            assert_eq!(context.render_commands.drain().count(), 0);
            assert_eq!(context.render_commands.pending_texture_destroys().len(), usize::from(auto_release));
        }
    }

    #[test]
    fn despawn_releases_shared_model_only_after_last_instance() {
        for auto_release in [true, false] {
            let mut context = TestContext::default();
            let handle = MeshHandle::new(0, VertexLayout::Mesh3D);
            let model = context.resources.register_gltf_model(
                "model", kani_volcano_animation::GltfModel::default(),
                vec![vec![handle]], vec![], vec![], auto_release,
            ).unwrap();
            let mesh = context.resources.gltf_model_asset(model).unwrap().meshes[0][0];
            let mut roots = Vec::new();
            for _ in 0..2 {
                let root = context.spawn();
                roots.push(root);
                context.add_component(root, GltfModelRoot::new(model));
                context.resources.retain_gltf_model(model).unwrap();
                let node = context.spawn();
                context.add_component(node, ParentComponent::new(root));
                // Two renderers sharing one mesh must each release their reference.
                for _ in 0..2 {
                    let child = context.spawn();
                    context.add_component(child, ParentComponent::new(node));
                    context.resources.retain_mesh(mesh);
                    context.add_component(child, mesh_renderer(Some(mesh)));
                }
            }
            let unrelated = context.spawn();
            assert!(context.despawn(roots[0]));
            assert_eq!(context.entity_count(), 5);
            assert_eq!(context.resources.gltf_model_asset(model).unwrap().ref_count, 1);
            assert_eq!(context.resources.mesh_assets().next().unwrap().1.ref_count, 3);
            assert_eq!(context.render_commands.drain().count(), 0);
            assert!(context.despawn(roots[1]));
            assert_eq!(context.entities(), &[unrelated]);
            assert_eq!(context.render_commands.drain().count(), usize::from(auto_release));
            assert!(!context.despawn(roots[1]));
            assert_eq!(context.render_commands.drain().count(), 0);
            if auto_release {
                assert!(context.resources.gltf_model_asset(model).is_none());
                assert!(context.resources.get_mesh_handle(mesh).is_none());
            } else {
                assert_eq!(context.resources.gltf_model_asset(model).unwrap().ref_count, 0);
                assert_eq!(context.resources.mesh_assets().next().unwrap().1.ref_count, 1);
            }
        }
    }

    #[test]
    fn despawn_handles_parent_cycles_without_double_release() {
        let mut context = TestContext::default();
        let first = context.spawn();
        let second = context.spawn();
        context.add_component(first, ParentComponent::new(second));
        context.add_component(second, ParentComponent::new(first));
        assert!(context.despawn(first));
        assert_eq!(context.entity_count(), 0);
        assert!(!context.despawn(first));
    }

    #[test]
    fn mesh_asset_id_returns_the_requested_entity_asset() {
        let mut context = TestContext::default();
        let first = context.spawn();
        let second = context.spawn();
        context.add_component(first, mesh_renderer(Some(MeshAssetId(3))));
        context.add_component(second, mesh_renderer(Some(MeshAssetId(7))));

        assert_eq!(context.mesh_asset_id(first), Some(MeshAssetId(3)));
        assert_eq!(context.mesh_asset_id(second), Some(MeshAssetId(7)));
    }

    #[test]
    fn mesh_asset_id_returns_none_without_mesh_renderer() {
        let mut context = TestContext::default();
        let entity = context.spawn();

        assert_eq!(context.mesh_asset_id(entity), None);
    }

    #[test]
    fn mesh_asset_id_returns_none_without_asset_id() {
        let mut context = TestContext::default();
        let entity = context.spawn();
        context.add_component(entity, mesh_renderer(None));

        assert_eq!(context.mesh_asset_id(entity), None);
    }

    #[test]
    fn mesh_asset_id_returns_none_after_entity_is_despawned() {
        let mut context = TestContext::default();
        let entity = context.spawn();
        context.add_component(entity, mesh_renderer(Some(MeshAssetId(3))));
        assert!(context.despawn(entity));

        assert_eq!(context.mesh_asset_id(entity), None);
    }
}

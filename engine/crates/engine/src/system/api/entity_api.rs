use crate::{
    Component, ComponentPool, EntityId, MeshRenderer, RenderCommandQueue, Resources, World,
    MeshAssetId, GltfModelRoot, ParentComponent, PendingPrimitiveMesh,
};
use std::collections::{HashMap, HashSet};
use std::any::{Any, TypeId};

fn owns_assets<T: Component>() -> bool {
    let type_id = TypeId::of::<T>();
    type_id == TypeId::of::<MeshRenderer>()
        || type_id == TypeId::of::<PendingPrimitiveMesh>()
        || type_id == TypeId::of::<GltfModelRoot>()
}

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
            self.remove_component::<MeshRenderer>(current);
            self.remove_component::<PendingPrimitiveMesh>(current);
            self.remove_component::<GltfModelRoot>(current);
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

    /// Retains new ownership before releasing the replaced component's assets.
    /// Validation failure leaves the existing component unchanged.
    fn add_component<T: Component>(&mut self, entity: EntityId, component: T) -> anyhow::Result<()> {
        anyhow::ensure!(self.world().contains(entity), "Entity not found: {entity:?}");
        self.resources_mut().retain_component_assets(&component)?;
        self.remove_component::<T>(entity);
        self.world_mut().add_component(entity, component);
        Ok(())
    }

    /// Removes a component and releases its asset ownership. Internal ownership
    /// transfers must use World::remove_component instead.
    fn remove_component<T: Component>(&mut self, entity: EntityId) -> bool {
        let Some(component) = self.world_mut().remove_component::<T>(entity) else {
            return false;
        };
        let component = &component as &dyn Any;
        let material = if let Some(renderer) = component.downcast_ref::<MeshRenderer>() {
            if let Some(id) = renderer.asset_id {
                if let Some(mesh) = self.resources_mut().release_mesh(id) {
                    self.render_commands_mut().destroy_mesh(mesh);
                }
            }
            Some(&renderer.material)
        } else {
            component.downcast_ref::<PendingPrimitiveMesh>().map(|pending| &pending.material)
        };
        if let Some(material) = material {
            for texture in self.resources_mut().release_material_textures(material) {
                self.render_commands_mut().destroy_texture(texture);
            }
        }
        if let Some(root) = component.downcast_ref::<GltfModelRoot>() {
            let released = self.resources_mut().release_gltf_model(root.model_id);
            for mesh in released.meshes {
                self.render_commands_mut().destroy_mesh(mesh);
            }
            for texture in released.textures {
                self.render_commands_mut().destroy_texture(texture);
            }
        }
        true
    }


    fn get_component_pool<T: Component>(&self) -> Option<&ComponentPool<T>> {
        self.world().get_pool::<T>()
    }

    /// Asset-owning components return None; use the managed component/material APIs instead.
    fn get_component_pool_mut<T: Component>(&mut self) -> Option<&mut ComponentPool<T>> {
        if owns_assets::<T>() {
            return None;
        }
        self.world_mut().get_pool_mut::<T>()
    }

    fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        self.world().get_component::<T>(entity)
    }

    /// Asset-owning components return None; use set_material/add_component instead.
    fn get_component_mut<T: Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        if owns_assets::<T>() {
            return None;
        }
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

    /// Returns an empty iterator if A owns assets.
    fn query1_mut<A>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A)> + '_>
    where
        A: Component,
    {
        if owns_assets::<A>() {
            return Box::new(std::iter::empty());
        }
        self.world_mut().query1_mut::<A>()
    }

    fn query2<A, B>(&self) -> Box<dyn Iterator<Item = (EntityId, &A, &B)> + '_>
    where
        A: Component,
        B: Component,
    {
        self.world().query2::<A, B>()
    }

    /// Returns an empty iterator if A owns assets. Read-only B is unrestricted.
    fn query2_mut<A, B>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A, &B)> + '_>
    where
        A: Component,
        B: Component,
    {
        if owns_assets::<A>() {
            return Box::new(std::iter::empty());
        }
        self.world_mut().query2_mut::<A, B>()
    }

    /// Returns an empty iterator if either component owns assets.
    fn query2_mut_mut<A, B>(&mut self) -> Box<dyn Iterator<Item = (EntityId, &mut A, &mut B)> + '_>
    where
        A: Component,
        B: Component,
    {
        if owns_assets::<A>() || owns_assets::<B>() {
            return Box::new(std::iter::empty());
        }
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
            mesh: MeshHandle::new(0, VertexLayout::Lit3D),
            asset_id,
            material: Material::default(),
        }
    }

    #[test]
    fn managed_components_allow_reading_but_reject_mutable_access() {
        fn check<T: Component>(component: T) {
            let mut context = TestContext::default();
            let entity = context.spawn();
            // Populate raw storage to ensure rejection is not just a missing pool/entity.
            context.world.add_component(entity, component);
            context.add_component(entity, crate::Visibility::default()).unwrap();
            assert!(context.get_component::<T>(entity).is_some());
            assert!(context.get_component_pool::<T>().is_some());
            assert_eq!(context.query1::<T>().count(), 1);
            assert!(context.get_component_mut::<T>(entity).is_none());
            assert!(context.get_component_pool_mut::<T>().is_none());
            assert_eq!(context.query1_mut::<T>().count(), 0);
            assert_eq!(context.query2_mut::<T, crate::Visibility>().count(), 0);
            assert_eq!(context.query2_mut_mut::<T, crate::Visibility>().count(), 0);
            assert_eq!(context.query2_mut_mut::<crate::Visibility, T>().count(), 0);
            for (_, visibility, _) in context.query2_mut::<crate::Visibility, T>() {
                visibility.is_visible = false;
            }
            assert!(!context.get_component::<crate::Visibility>(entity).unwrap().is_visible);
            assert!(context.get_component_mut::<crate::Visibility>(entity).is_some());
            assert!(context.get_component_pool_mut::<crate::Visibility>().is_some());
            assert_eq!(context.query1_mut::<crate::Visibility>().count(), 1);
        }
        check(mesh_renderer(None));
        check(PendingPrimitiveMesh {
            shape: crate::PrimitiveShape::Sphere { radius: 1.0, rings: 8, segments: 8,
                color: Material::default().base_color.truncate() },
            material: Material::default(), auto_release: true,
        });
        check(GltfModelRoot::new(crate::GltfModelAssetId(0)));
    }

    #[test]
    fn replacing_renderer_preserves_shared_assets_and_rejects_invalid_ownership() {
        let mut context = TestContext::default();
        let entity = context.spawn();
        let mesh = context.resources.insert_mesh_asset(mesh_renderer(None).mesh, true);
        let texture = context.resources.insert_texture_asset(renderer_vulkan::TextureHandle(1), true);
        let renderer = MeshRenderer {
            material: Material { base_color_texture: Some(texture), emissive_texture: Some(texture),
                ..Material::default() },
            ..mesh_renderer(Some(mesh))
        };
        context.add_component(entity, renderer.clone()).unwrap();
        context.add_component(entity, renderer.clone()).unwrap();
        assert_eq!(context.resources.mesh_assets().next().unwrap().1.ref_count, 1);
        assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 1);
        assert_eq!(context.render_commands.drain().count(), 0);

        let mut invalid = renderer.clone();
        invalid.material.normal_texture = Some(crate::TextureAssetId(99));
        assert!(context.add_component(entity, invalid).is_err());
        let mut invalid = renderer.clone();
        invalid.mesh = MeshHandle::new(99, VertexLayout::Lit3D);
        assert!(context.add_component(entity, invalid).is_err());
        assert!(context.add_component(EntityId(999), renderer.clone()).is_err());
        assert_eq!(context.get_component::<MeshRenderer>(entity).unwrap().material, renderer.material);
        assert_eq!(context.resources.mesh_assets().next().unwrap().1.ref_count, 1);
        assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 1);

        let new_handle = MeshHandle::new(1, VertexLayout::Lit3D);
        let new_mesh = context.resources.insert_mesh_asset(new_handle, true);
        context.add_component(entity, MeshRenderer::new(new_handle, Material::default()).unwrap()
            .with_asset_id(new_mesh)).unwrap();
        assert!(context.resources.get_mesh_handle(mesh).is_none());
        assert!(context.resources.get_texture_handle(texture).is_none());
        assert_eq!(context.resources.mesh_assets().next().unwrap().1.ref_count, 1);
        assert_eq!(context.render_commands.drain().count(), 2);
        assert!(context.despawn(entity));
        assert!(context.resources.get_mesh_handle(new_mesh).is_none());
        assert_eq!(context.render_commands.drain().count(), 1);
    }

    #[test]
    fn pending_to_renderer_transfers_texture_ownership_without_destroying_it() {
        let mut context = TestContext::default();
        let entity = context.spawn();
        let texture = context.resources.insert_texture_asset(renderer_vulkan::TextureHandle(1), true);
        let material = Material { base_color_texture: Some(texture), ..Material::default() };
        context.add_component(entity, PendingPrimitiveMesh {
            shape: crate::PrimitiveShape::Sphere { radius: 1.0, rings: 8, segments: 8,
                color: material.base_color.truncate() },
            material, auto_release: true,
        }).unwrap();
        let mesh = context.resources.insert_mesh_asset(mesh_renderer(None).mesh, true);
        context.add_component(entity, MeshRenderer { material, ..mesh_renderer(Some(mesh)) }).unwrap();
        assert!(context.remove_component::<PendingPrimitiveMesh>(entity));
        assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 1);
        assert_eq!(context.resources.mesh_assets().next().unwrap().1.ref_count, 1);
        assert_eq!(context.render_commands.drain().count(), 0);
        assert!(context.despawn(entity));
        assert!(context.resources.get_texture_handle(texture).is_none());
        assert_eq!(context.render_commands.drain().count(), 2);
    }

    #[test]
    fn removing_material_components_then_despawning_releases_only_once() {
        for pending in [false, true] {
            let mut context = TestContext::default();
            let handle = renderer_vulkan::TextureHandle(1);
            let texture = context.resources.insert_texture_asset(handle, true);
            let material = Material { base_color_texture: Some(texture),
                emissive_texture: Some(texture), ..Material::default() };
            // A separate owner must survive both removal and subsequent despawn.
            context.resources.retain_material_textures(&material).unwrap();
            let entity = context.spawn();
            if pending {
                context.add_component(entity, PendingPrimitiveMesh {
                    shape: crate::PrimitiveShape::Sphere { radius: 1.0, rings: 8, segments: 8,
                        color: material.base_color.truncate() },
                    material, auto_release: true,
                }).unwrap();
                assert!(context.remove_component::<PendingPrimitiveMesh>(entity));
                assert!(!context.remove_component::<PendingPrimitiveMesh>(entity));
            } else {
                let mesh_handle = MeshHandle::new(0, VertexLayout::Lit3D);
                let mesh = context.resources.insert_mesh_asset(mesh_handle, true);

                context.add_component(entity, MeshRenderer { material, ..mesh_renderer(Some(mesh)) }).unwrap();
                assert!(context.remove_component::<MeshRenderer>(entity));
                assert!(!context.remove_component::<MeshRenderer>(entity));
                assert!(context.resources.get_mesh_handle(mesh).is_none());
                let commands: Vec<_> = context.render_commands.drain().collect();
                assert!(matches!(commands.as_slice(),
                    [crate::RenderCommand::DestroyMesh { mesh }] if *mesh == mesh_handle));
            }
            assert!(context.is_entity_registered(entity));
            assert!(context.despawn(entity));
            assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 1);
            assert_eq!(context.render_commands.drain().count(), 0);
            assert_eq!(context.resources.release_material_textures(&material), vec![handle]);
        }
    }

    #[test]
    fn removing_regular_components_keeps_entity_alive() {
        let mut context = TestContext::default();
        let entity = context.spawn();
        context.add_component(entity, crate::Visibility::default()).unwrap();
        assert!(context.remove_component::<crate::Visibility>(entity));
        assert!(!context.remove_component::<crate::Visibility>(entity));
        assert!(context.is_entity_registered(entity));
        assert_eq!(context.render_commands.drain().count(), 0);
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
            context.add_component(root, MeshRenderer { material, ..mesh_renderer(None) }).unwrap();
            context.add_component(unrelated, MeshRenderer { material, ..mesh_renderer(None) }).unwrap();
            context.add_component(child, ParentComponent::new(root)).unwrap();
            context.add_component(child, PendingPrimitiveMesh {
                shape: crate::PrimitiveShape::Sphere { radius: 1.0, rings: 8, segments: 8,
                    color: material.base_color.truncate() },
                material,
                auto_release: true,
            }).unwrap();
            assert!(context.despawn(root));
            assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 1);
            assert_eq!(context.render_commands.drain().count(), 0);
            assert!(!context.despawn(root));
            assert!(context.despawn(unrelated));
            let commands: Vec<_> = context.render_commands.drain().collect();
            if auto_release {
                assert!(context.resources.get_texture_handle(texture).is_none());
                assert!(matches!(commands.as_slice(),
                    [crate::RenderCommand::DestroyTexture { texture }] if *texture == handle));
            } else {
                assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, 0);
                assert!(commands.is_empty());
            }
            assert!(!context.despawn(unrelated));
            // Repeated despawn must not queue destruction twice.
            assert_eq!(context.render_commands.drain().count(), 0);
        }
    }

    #[test]
    fn despawn_releases_shared_model_only_after_last_instance() {
        for auto_release in [true, false] {
            let mut context = TestContext::default();
            let handle = MeshHandle::new(0, VertexLayout::Lit3D);
            let model = context.resources.register_gltf_model(
                "model", kani_volcano_animation::GltfModel::default(),
                vec![vec![handle]], vec![], vec![], auto_release,
            ).unwrap();
            let mesh = context.resources.gltf_model_asset(model).unwrap().meshes[0][0];
            let mut roots = Vec::new();
            for _ in 0..2 {
                let root = context.spawn();
                roots.push(root);
                context.add_component(root, GltfModelRoot::new(model)).unwrap();
                let node = context.spawn();
                context.add_component(node, ParentComponent::new(root)).unwrap();
                // Two renderers sharing one mesh must each release their reference.
                for _ in 0..2 {
                    let child = context.spawn();
                    context.add_component(child, ParentComponent::new(node)).unwrap();

                    context.add_component(child, mesh_renderer(Some(mesh))).unwrap();
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
        context.add_component(first, ParentComponent::new(second)).unwrap();
        context.add_component(second, ParentComponent::new(first)).unwrap();
        assert!(context.despawn(first));
        assert_eq!(context.entity_count(), 0);
        assert!(!context.despawn(first));
    }

    #[test]
    fn mesh_asset_id_returns_the_requested_entity_asset() {
        let mut context = TestContext::default();
        let first = context.spawn();
        let second = context.spawn();
        let first_mesh = context.resources.insert_mesh_asset(mesh_renderer(None).mesh, true);
        let second_mesh = context.resources.insert_mesh_asset(mesh_renderer(None).mesh, true);
        context.add_component(first, mesh_renderer(Some(first_mesh))).unwrap();
        context.add_component(second, mesh_renderer(Some(second_mesh))).unwrap();

        assert_eq!(context.mesh_asset_id(first), Some(first_mesh));
        assert_eq!(context.mesh_asset_id(second), Some(second_mesh));
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
        context.add_component(entity, mesh_renderer(None)).unwrap();

        assert_eq!(context.mesh_asset_id(entity), None);
    }

    #[test]
    fn mesh_asset_id_returns_none_after_entity_is_despawned() {
        let mut context = TestContext::default();
        let entity = context.spawn();
        let mesh = context.resources.insert_mesh_asset(mesh_renderer(None).mesh, true);
        context.add_component(entity, mesh_renderer(Some(mesh))).unwrap();
        assert!(context.despawn(entity));

        assert_eq!(context.mesh_asset_id(entity), None);
    }
}

use anyhow::{Result, anyhow, bail, Context, ensure};
use cgmath::{vec3, InnerSpace, Matrix3, Matrix4, Quaternion};
use kani_volcano_animation::NodeTransform;
use kani_volcano_math::Transform;
use renderer_vulkan::PipelineKey;

use crate::{
    AssetApi, EntityApi, EntityId, Material, MeshAssetId, MeshRenderer, PrimitiveShape, Visibility,
    DrawOrder, GltfModelAssetId, GltfModelRoot, ParentComponent,
};
use crate::{
    Text, TextPipeline,
};


pub type Vec3 = cgmath::Vector3<f32>;
pub type Vec2 = cgmath::Vector2<f32>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Input, Resources, SceneContext, SceneId, Scheduler, Time, World, TextureAssetId};
    use kani_volcano_animation::{GltfModel, ModelScene, ModelNode, ModelMesh, ModelPrimitive};
    use renderer_vulkan::{MeshHandle, TextureHandle, VertexLayout};

    fn with_context(test: impl FnOnce(&mut SceneContext<'_>)) {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut scheduler = Scheduler::default();
        let input = Input::default();
        let time = Time::default();
        test(&mut SceneContext::new(SceneId(1), &mut world, &input, &time,
            &mut resources, &mut scheduler));
    }

    fn texture_count(context: &SceneContext<'_>, id: TextureAssetId) -> usize {
        AssetApi::resources(context).texture_assets().find(|(key, _)| *key == id).unwrap().1.ref_count
    }

    #[test]
    fn model_spawn_retains_textures_only_after_validation() {
        with_context(|context| {
            let resources = AssetApi::resources_mut(context);
            let mesh = resources.register_model("model", MeshHandle::new(0, VertexLayout::Lit3D), true);
            let texture = resources.insert_texture_asset(TextureHandle(1), true);
            let material = Material { base_color_texture: Some(texture),
                emissive_texture: Some(texture), ..Material::default() };
            let invalid = Material { normal_texture: Some(TextureAssetId(99)), ..material };
            assert!(context.spawn_model("model", Transform::default(), invalid).is_err());
            let resources = AssetApi::resources(context);
            assert_eq!(resources.mesh_assets().find(|(id, _)| *id == mesh).unwrap().1.ref_count, 0);
            assert_eq!(texture_count(context, texture), 0);
            assert_eq!(context.query1::<MeshRenderer>().count(), 0);
            context.spawn_model("model", Transform::default(), material).unwrap();
            assert_eq!(texture_count(context, texture), 1);
            assert_eq!(context.query1::<MeshRenderer>().count(), 1);
        });
    }

    #[test]
    fn gltf_spawn_retains_per_primitive_and_rolls_back_invalid_textures() {
        with_context(|context| {
            let resources = AssetApi::resources_mut(context);
            let texture = resources.insert_texture_asset(TextureHandle(1), true);
            let material = Material { base_color_texture: Some(texture),
                emissive_texture: Some(texture), ..Material::default() };
            let source = GltfModel {
                scenes: vec![ModelScene { roots: vec![0], ..Default::default() }],
                nodes: vec![ModelNode { mesh: Some(0), ..Default::default() }],
                meshes: vec![ModelMesh { primitives: (0..2).map(|_| ModelPrimitive {
                    material: Some(0), ..Default::default()
                }).collect(), ..Default::default() }],
                ..Default::default()
            };
            let model = resources.register_gltf_model("model", source,
                vec![vec![MeshHandle::new(0, VertexLayout::Lit3D), MeshHandle::new(1, VertexLayout::Lit3D)]],
                vec![material], vec![crate::resources::GltfTextureAsset { srgb: Some(texture), linear: None }], true).unwrap();
            let invalid = Material { normal_texture: Some(TextureAssetId(99)), ..material };
            assert!(context.spawn_gltf_model("model", Transform::default(), Some(invalid)).is_err());
            assert_eq!(texture_count(context, texture), 1);
            assert_eq!(context.query1::<MeshRenderer>().count(), 0);
            assert_eq!(AssetApi::resources(context).gltf_model_asset(model).unwrap().ref_count, 0);
            context.spawn_gltf_model("model", Transform::default(), None).unwrap();
            assert_eq!(texture_count(context, texture), 3);
            context.spawn_gltf_model("model", Transform::default(), Some(material)).unwrap();
            assert_eq!(texture_count(context, texture), 5);
        });
    }
}

fn spawn_gltf_node<A: EntityApi + ?Sized>(
    api: &mut A,
    model_id: GltfModelAssetId,
    index: usize,
    parent: EntityId,
    material_override: Option<Material>,
    created: &mut Vec<EntityId>,
    visited: &mut std::collections::HashSet<usize>,
) -> Result<()> {
    ensure!(visited.insert(index), "Repeated or cyclic glTF node: {index}");
    // Copy this node's instance data so Resources is no longer borrowed when spawning.
    let (transform, children, primitives) = {
        let resources = api.resources();
        let asset = resources.gltf_model_asset(model_id).context("glTF model asset not found")?;
        let node = asset.source.nodes.get(index).context("Invalid glTF node index")?;
        ensure!(node.skin.is_none(), "Skinned nodes are not supported yet");
        let mut primitives = Vec::new();
        if let Some(mesh_index) = node.mesh {
            let mesh = asset.source.meshes.get(mesh_index).context("Invalid glTF mesh index")?;
            let ids = asset.meshes.get(mesh_index).context("Missing uploaded mesh")?;
            ensure!(ids.len() == mesh.primitives.len(), "Primitive/mesh asset counts differ");
            for (primitive, &id) in mesh.primitives.iter().zip(ids) {
                let material = match material_override {
                    Some(material) => material,
                    None => match primitive.material {
                        Some(index) => *asset.materials.get(index).context("Invalid material index")?,
                        None => Material {
                            metallic: 1.0, roughness: 1.0,
                            pipeline_key: PipelineKey::Pbr3D,
                            ..Material::default()
                        },
                    },
                };
                let handle = resources.get_mesh_handle(id).context("Mesh asset not found")?;
                primitives.push(MeshRenderer::new(handle, material)?.with_asset_id(id));
            }
        }
        (convert_node_transform(&node.transform)?, node.children.clone(), primitives)
    };
    let entity = api.spawn();
    created.push(entity);
    api.add_component(entity, ParentComponent::new(parent));
    api.add_component(entity, transform);
    for renderer in primitives {
        let id = renderer.asset_id.expect("prepared renderer has an asset ID");
        let count = api.resources().mesh_assets().find(|(key, _)| *key == id)
            .context("Mesh asset not found")?.1.ref_count;
        ensure!(count < usize::MAX, "Mesh reference count exhausted");
        api.resources_mut().retain_mesh(id).context("Mesh asset not found")?;
        let primitive = api.spawn();
        created.push(primitive);
        api.add_component(primitive, ParentComponent::new(entity));
        api.add_component(primitive, Transform::default());
        api.add_component(primitive, renderer);
        api.add_component(primitive, Visibility::default());
    }
    for child in children {
        spawn_gltf_node(api, model_id, child, entity, material_override, created, visited)?;
    }
    Ok(())
}

fn convert_node_transform(source: &NodeTransform) -> Result<Transform> {
    let (position, scale, rotation, expected) = match source {
        NodeTransform::Trs {
            translation,
            rotation,
            scale,
        } => {
            let q = Quaternion::new(rotation[3], rotation[0], rotation[1], rotation[2]);
            ensure!(
                q.magnitude2().is_finite() && q.magnitude2() > 0.0,
                "Invalid quaternion"
            );
            let rotation = Matrix3::from(q.normalize());
            let position = vec3(translation[0], translation[1], translation[2]);
            let scale = vec3(scale[0], scale[1], scale[2]);
            let expected = Matrix4::from_translation(position)
                * Matrix4::from(rotation)
                * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
            (position, scale, rotation, expected)
        }
        NodeTransform::Matrix(values) => {
            let matrix = Matrix4::from(*values);
            let x = matrix.x.truncate();
            let y = matrix.y.truncate();
            let z = matrix.z.truncate();
            let mut scale = vec3(x.magnitude(), y.magnitude(), z.magnitude());
            ensure!(
                scale.x > 0.0 && scale.y > 0.0 && scale.z > 0.0,
                "Zero-scale matrix transforms cannot be decomposed; use TRS"
            );
            if x.cross(y).dot(z) < 0.0 {
                scale.x = -scale.x;
            }
            let rotation = Matrix3::from_cols(x / scale.x, y / scale.y, z / scale.z);
            (matrix.w.truncate(), scale, rotation, matrix)
        }
    };
    // Transform::matrix uses Rx * Ry * Rz, in degrees.
    let sin_y = rotation.z.x.clamp(-1.0, 1.0);
    let cos_y = rotation.x.x.hypot(rotation.y.x);
    let y = sin_y.atan2(cos_y);
    let (x, z) = if cos_y > 1.0e-6 {
        (
            (-rotation.z.y).atan2(rotation.z.z),
            (-rotation.y.x).atan2(rotation.x.x),
        )
    } else {
        ((sin_y * rotation.x.y).atan2(rotation.y.y), 0.0)
    };
    let result = Transform {
        position,
        scale,
        rotation: vec3(x.to_degrees(), y.to_degrees(), z.to_degrees()),
    };
    let actual: [[f32; 4]; 4] = result.matrix().into();
    let expected: [[f32; 4]; 4] = expected.into();
    ensure!(
        actual
            .iter()
            .flatten()
            .zip(expected.iter().flatten())
            .all(|(&a, &b)| a.is_finite()
                && b.is_finite()
                && (a - b).abs() <= 1.0e-4 * b.abs().max(1.0)),
        "Node transform cannot be represented by the current TRS Transform"
    );
    Ok(result)
}


pub trait ObjectApi: EntityApi + AssetApi {
    /// Replaces the material without rebuilding an existing mesh or its vertex colors.
    /// Pending primitives use the new material's vertex layout when generated.
    /// An incompatible existing mesh returns an error without changing its material.
    fn set_material(&mut self, entity: EntityId, material: Material) -> Result<()> {
        anyhow::ensure!(self.is_entity_registered(entity), "entity not found: {entity:?}");

        if let Some(renderer) = self.get_component::<MeshRenderer>(entity) {
            anyhow::ensure!(
                renderer.mesh.vertex_layout == material.pipeline_key.required_vertex_layout(),
                "Material pipeline {:?} does not match mesh vertex layout {:?}.",
                material.pipeline_key,
                renderer.mesh.vertex_layout
            );
        } else if !self.has_component::<crate::PendingPrimitiveMesh>(entity) {
            bail!("entity {entity:?} has no MeshRenderer or PendingPrimitiveMesh");
        }

        if let Some(renderer) = self.get_component_mut::<MeshRenderer>(entity) {
            renderer.material = material;
        }
        if let Some(pending) = self.get_component_mut::<crate::PendingPrimitiveMesh>(entity) {
            pending.material = material;
        }
        Ok(())
    }

    fn spawn_model(
        &mut self,
        model_name: &str,
        transform: Transform,
        material: Material,
    ) -> Result<EntityId> {
        let asset_id = self.model_asset_id(model_name)?;

        let mesh = AssetApi::resources(self)
            .get_mesh_handle(asset_id)
            .ok_or_else(|| anyhow!("mesh asset not found: {asset_id:?}"))?;

        let mesh_renderer = MeshRenderer::new(mesh, material)?.with_asset_id(asset_id);
        let resources = AssetApi::resources_mut(self);
        let count = resources.mesh_assets().find(|(id, _)| *id == asset_id)
            .context("Mesh asset not found")?.1.ref_count;
        ensure!(count < usize::MAX, "Mesh reference count exhausted");
        resources.retain_material_textures(&material)?;
        resources.retain_mesh(asset_id);

        let entity = self.spawn();

        self.add_component(entity, transform);
        self.add_component(entity, mesh_renderer);
        self.add_component(entity, Visibility::default());
        self.set_tags(entity, ["Model", model_name]);

        Ok(entity)
    }

    /// Spawns the default scene (or first scene) with local node transforms.
    /// None uses imported materials; Some overrides every primitive.
    fn spawn_gltf_model(
        &mut self,
        model_name: &str,
        transform: Transform,
        material: Option<Material>,
    ) -> Result<EntityId>{
        let asset_id=self.gltf_model_asset_id(model_name)?;

        // get all nodes from meshes[0]
        let roots = {
            let asset = AssetApi::resources(self).gltf_model_asset(asset_id)
                .context("glTF model asset not found")?;
            asset.source.scenes.get(asset.source.default_scene.unwrap_or(0))
                .context("glTF model has no valid scene")?.roots.clone()
        };
        let root = self.spawn();
        self.add_component(root, GltfModelRoot::new(asset_id));
        self.add_component(root, transform);
        let mut created = vec![root];
        let mut visited = std::collections::HashSet::new();
        let result = (|| {
            for node in roots {
                spawn_gltf_node(self, asset_id, node, root, material, &mut created, &mut visited)?;
            }
            // Validate the whole instance before retaining any textures, so a failed
            // spawn does not release (and auto-remove) caller-owned texture assets.
            let mut texture_counts = std::collections::HashMap::<crate::TextureAssetId, usize>::new();
            for &entity in &created {
                if let Some(renderer) = self.get_component::<MeshRenderer>(entity) {
                    let material = &renderer.material;
                    let ids = [material.base_color_texture, material.metallic_roughness_texture,
                        material.normal_texture, material.emissive_texture, material.occlusion_texture];
                    for (index, id) in ids.iter().enumerate() {
                        if let Some(id) = id {
                            if !ids[..index].contains(&Some(*id)) {
                                let count = texture_counts.entry(*id).or_default();
                                *count = count.checked_add(1).context("Texture reference count exhausted")?;
                            }
                        }
                    }
                }
            }
            if !texture_counts.is_empty() {
                for (id, asset) in AssetApi::resources(self).texture_assets() {
                    if let Some(count) = texture_counts.remove(&id) {
                        ensure!(asset.ref_count.checked_add(count).is_some(), "Texture reference count exhausted: {id:?}");
                    }
                    if texture_counts.is_empty() { break; }
                }
                ensure!(texture_counts.is_empty(), "Texture asset not found");
            }
            AssetApi::resources_mut(self).retain_gltf_model(asset_id)?;
            for &entity in &created {
                if let Some(material) = self.get_component::<MeshRenderer>(entity).map(|renderer| renderer.material) {
                    // All IDs and cumulative increments were validated above.
                    AssetApi::resources_mut(self).retain_material_textures(&material)
                        .expect("validated texture ownership");
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            for entity in created.into_iter().rev() {
                if let Some(renderer) = self.world_mut().remove_component::<MeshRenderer>(entity) {
                    if let Some(id) = renderer.asset_id {
                        if let Some(handle) = AssetApi::resources_mut(self).release_mesh(id) {
                            EntityApi::render_commands_mut(self).destroy_mesh(handle);
                        }
                    }
                }
                // Roll back only this spawn; do not call the public despawn path.
                self.world_mut().despawn(entity);
            }
            return Err(error);
        }
        Ok(root)
    }


    fn spawn_text_ui2d(
        &mut self,
        font_name: &str,
        text: &str,
        font_size: f32,
        line_height: f32,
        position: Vec2,
        scale: Vec2,
        rotation: f32,
        color: Vec3,
        alpha: f32,
    ) -> Result<EntityId> {
        anyhow::ensure!(
            font_size.is_finite() && font_size > 0.0,
            "font size must be finite and positive"
        );
        anyhow::ensure!(
            line_height.is_finite() && line_height > 0.0,
            "line height must be finite and positive"
        );
        anyhow::ensure!(
            alpha.is_finite() && (0.0..=1.0).contains(&alpha),
            "alpha must be between 0 and 1"
        );

        let font = self.font_asset_id(font_name)?;

        let entity = self.spawn();

        self.add_component(entity, Text {
            content: text.to_string(),
            font,
            font_size,
            line_height,
            color,
            alpha,
            text_pipeline: TextPipeline::Ui2D,
        });

        let transform=Transform{
            position: vec3(position.x, position.y,0.0),
            rotation: vec3(0.0,0.0,rotation),
            scale: vec3(scale.x,scale.y,0.0),
        };

        self.add_component(entity, transform);
        self.add_component(entity, Visibility::default());

        Ok(entity)
    }

    fn spawn_text_3d(
        &mut self,
        font_name: &str,
        text: &str,
        font_size: f32,
        line_height: f32,
        position: Vec3,
        scale: Vec3,
        rotation: Vec3,
        color: Vec3,
        alpha: f32,
    ) -> Result<EntityId>{
        anyhow::ensure!(
            font_size.is_finite() && font_size > 0.0,
            "font size must be finite and positive"
        );
        anyhow::ensure!(
            line_height.is_finite() && line_height > 0.0,
            "line height must be finite and positive"
        );
        anyhow::ensure!(
            alpha.is_finite() && (0.0..=1.0).contains(&alpha),
            "alpha must be between 0 and 1"
        );

        let font=self.font_asset_id(font_name)?;

        let entity=self.spawn();

        self.add_component(entity, Text{
            content: text.to_string(),
            font,
            font_size,
            line_height,
            color,
            alpha,
            text_pipeline: TextPipeline::World3D,
        });

        let transform=Transform{
            position: position,
            rotation: rotation,
            scale: scale,
        };

        self.add_component(entity,transform);
        self.add_component(entity,Visibility::default());

        Ok(entity)
    }


    fn spawn_primitive_from_mesh(
        &mut self,
        asset_id: MeshAssetId,
        material: Material,
        transform: Transform,
    ) -> Result<EntityId>;

    fn primitive_material(
        &self,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<Material>;

    fn spawn_shape_with_material(
        &mut self,
        shape: PrimitiveShape,
        transform: Transform,
        material: Material,
        auto_release: bool,
    ) -> Result<EntityId>;

    fn spawn_triangle_3d(
        &mut self,
        p0: Vec3,
        p1: Vec3,
        p2: Vec3,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<EntityId> {
        let material = self.primitive_material(color, alpha, texture, pipeline_key)?;
        let center = (p0 + p1 + p2) / 3.0;
        let shape = PrimitiveShape::Triangle {
            points: [p0 - center, p1 - center, p2 - center],
        color: material.base_color.truncate(),
        };
        let transform = Transform {
            position: center,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_triangle_2d(
        &mut self,
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
    ) -> Result<EntityId> {
        self.spawn_triangle_3d(
            vec3(0.0, p0.x, p0.y),
            vec3(0.0, p1.x, p1.y),
            vec3(0.0, p2.x, p2.y),
            color,
            alpha,
            texture,
            PipelineKey::Ui2D,
        )
    }

    fn spawn_rectangle_3d(
        &mut self,
        pos: Vec3,
        width: f32,
        height: f32,
        rotation: Vec3,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<EntityId> {
        let material = self.primitive_material(color, alpha, texture, pipeline_key)?;
        let half_width = width * 0.5;
        let half_height = height * 0.5;
        let shape = PrimitiveShape::Rectangle {
            points: [
                vec3(0.0, -half_width, half_height),
                vec3(0.0, -half_width, -half_height),
                vec3(0.0, half_width, -half_height),
                vec3(0.0, half_width, half_height),
            ],
        color: material.base_color.truncate(),
        };
        let transform = Transform {
            position: pos,
            rotation,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_rectangle_2d(
        &mut self,
        pos: Vec2,
        width: f32,
        height: f32,
        rotation: f32,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
    ) -> Result<EntityId> {
        self.spawn_rectangle_3d(
            vec3(0.0, pos.x, pos.y),
            width,
            height,
            vec3(rotation, 0.0, 0.0),
            color,
            alpha,
            texture,
            PipelineKey::Ui2D,
        )
    }

    fn spawn_cube_3d(
        &mut self,
        pos: Vec3,
        length: f32,
        rotation: Vec3,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<EntityId> {
        let material = self.primitive_material(color, alpha, texture, pipeline_key)?;
        let h = length * 0.5;
        let shape = PrimitiveShape::Cube {
            points: [
                vec3(h, -h, h),
                vec3(h, h, h),
                vec3(-h, h, h),
                vec3(-h, -h, h),
                vec3(h, -h, -h),
                vec3(h, h, -h),
                vec3(-h, h, -h),
                vec3(-h, -h, -h),
            ],
        color: material.base_color.truncate(),
        };
        let transform = Transform {
            position: pos,
            rotation,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_cuboid_3d(
        &mut self,
        pos: Vec3,
        width: f32,
        depth: f32,
        height: f32,
        rotation: Vec3,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<EntityId> {
        let material =
            self.primitive_material(color, alpha, texture, pipeline_key)?;

        let w = width * 0.5;
        let d = depth * 0.5;
        let h = height * 0.5;

        let shape = PrimitiveShape::Cube {
            points: [
                vec3(w, -h, d),
                vec3(w, h, d),
                vec3(-w, h, d),
                vec3(-w, -h, d),
                vec3(w, -h, -d),
                vec3(w, h, -d),
                vec3(-w, h, -d),
                vec3(-w, -h, -d),
            ],
        color: material.base_color.truncate(),
        };

        let transform = Transform {
            position: pos,
            rotation,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_circle_3d(
        &mut self,
        pos: Vec3,
        radius: f32,
        segments: u32,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<EntityId> {
        let material = self.primitive_material(color, alpha, texture, pipeline_key)?;
        let shape = PrimitiveShape::Circle {
            radius,
            segments,
        color: material.base_color.truncate(),
        };
        let transform = Transform {
            position: pos,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_circle_2d(
        &mut self,
        pos: Vec2,
        radius: f32,
        segments: u32,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
    ) -> Result<EntityId> {
        self.spawn_circle_3d(
            vec3(0.0, pos.x, pos.y),
            radius,
            segments,
            color,
            alpha,
            texture,
            PipelineKey::Ui2D,
        )
    }

    fn spawn_polygon_3d(
        &mut self,
        points: Vec<Vec3>,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<EntityId> {
        if points.is_empty() {
            bail!("Polygon must have at least one point.");
        }

        let material = self.primitive_material(color, alpha, texture, pipeline_key)?;
        let center = points.iter().copied().sum::<Vec3>() / points.len() as f32;
        let local_points = points.iter().map(|p| *p - center).collect::<Vec<_>>();
        let shape = PrimitiveShape::Polygon {
            points: local_points,
        color: material.base_color.truncate(),
        };
        let transform = Transform {
            position: center,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_polygon_2d(
        &mut self,
        points: Vec<Vec2>,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
    ) -> Result<EntityId> {
        let points = points.into_iter().map(|p| vec3(0.0, p.x, p.y)).collect();
        self.spawn_polygon_3d(points, color, alpha, texture, PipelineKey::Ui2D)
    }

    fn spawn_sphere_3d(
        &mut self,
        center: Vec3,
        radius: f32,
        rings: u32,
        segments: u32,
        color: Vec3,
        alpha: f32,
        texture: Option<&str>,
        pipeline_key: PipelineKey,
    ) -> Result<EntityId> {
        let material = self.primitive_material(color, alpha, texture, pipeline_key)?;
        let shape = PrimitiveShape::Sphere {
            radius,
            rings,
            segments,
        color: material.base_color.truncate(),
        };
        let transform = Transform {
            position: center,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_line_3d(
        &mut self,
        pos0: Vec3,
        pos1: Vec3,
        color: Vec3,
        alpha: f32,
    ) -> Result<EntityId> {
        let material = self.primitive_material(color, alpha, None, PipelineKey::DebugLine3D)?;
        let center = (pos0 + pos1) / 2.0;
        let shape = PrimitiveShape::Line {
            pos0: pos0 - center,
            pos1: pos1 - center,
        color: material.base_color.truncate(),
        };
        let transform = Transform {
            position: center,
            ..Default::default()
        };

        self.spawn_shape_with_material(shape, transform, material, true)
    }

    fn spawn_line_2d(
        &mut self,
        pos0: Vec2,
        pos1: Vec2,
        color: Vec3,
        width: f32,
        alpha: f32,
    ) -> Result<EntityId> {
        let from = vec3(0.0, pos0.x, pos0.y);
        let to = vec3(0.0, pos1.x, pos1.y);
        let center = (from + to) / 2.0;
        let delta = to - from;
        let length = (delta.y * delta.y + delta.z * delta.z).sqrt();
        let rotation = vec3((-delta.y).atan2(delta.z).to_degrees(), 0.0, 0.0);

        self.spawn_rectangle_3d(
            center,
            width,
            length,
            rotation,
            color,
            alpha,
            None,
            PipelineKey::Ui2D,
        )
    }

    fn set_draw_order(&mut self, entity: EntityId, order: i32) {
        self.add_component(entity, DrawOrder(order));
    }
}

use anyhow::{Result, anyhow, bail};
use cgmath::vec3;
use kani_volcano_math::Transform;
use renderer_vulkan::PipelineKey;

use crate::{
    AssetApi, EntityApi, EntityId, Material, MeshAssetId, MeshRenderer, PrimitiveShape, Visibility,
};
use crate::{
    Text,
};


pub type Vec3 = cgmath::Vector3<f32>;
pub type Vec2 = cgmath::Vector2<f32>;

pub trait ObjectApi: EntityApi + AssetApi {
    fn spawn_model(
        &mut self,
        model_name: &str,
        transform: Transform,
        material: Material,
    ) -> Result<EntityId> {
        let asset_id = self.model_asset_id(model_name)?;

        let mesh = AssetApi::resources_mut(self)
            .retain_mesh(asset_id)
            .ok_or_else(|| anyhow!("mesh asset not found: {asset_id:?}"))?;

        let mesh_renderer = match MeshRenderer::new(mesh, material) {
            Ok(mesh_renderer) => mesh_renderer.with_asset_id(asset_id),
            Err(error) => {
                AssetApi::resources_mut(self).release_mesh(asset_id);
                return Err(error);
            }
        };

        let entity = self.spawn();

        self.add_component(entity, transform);
        self.add_component(entity, mesh_renderer);
        self.add_component(entity, Visibility::default());
        self.set_tags(entity, ["Model", model_name]);

        Ok(entity)
    }

    fn spawn_text(
        &mut self,
        font_name: &str,
        text: &str,
        font_size: f32,
        line_height: f32,
        transform: Transform,
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
        });
        self.add_component(entity, transform);
        self.add_component(entity, Visibility::default());

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
            color: material.color,
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
            color: material.color,
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
            color: material.color,
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
            color: material.color,
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
            color: material.color,
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
            color: material.color,
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
            color: material.color,
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
            color: material.color,
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
}

use cgmath::{Vector3, Vector4, vec3, vec4};
use anyhow::{Result, anyhow};
use renderer_vulkan::{PipelineKey, RenderAlphaMode, RenderMaterial};
use crate::{Resources, TextureAssetId};

pub type Vec3 = Vector3<f32>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Material {
    pub base_color: Vector4<f32>,
    pub base_color_texture: Option<TextureAssetId>,

    pub metallic: f32,

    pub roughness: f32,

    pub metallic_roughness_texture: Option<TextureAssetId>,

    pub normal_texture: Option<TextureAssetId>,
    pub normal_scale: f32, 

    pub emissive_color: Vector3<f32>,
    pub emissive_strength: f32,
    pub emissive_texture: Option<TextureAssetId>,

    pub occlusion_strength: f32,
    pub occlusion_texture: Option<TextureAssetId>,

    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,

    pub double_sided: bool,

    pub pipeline_key: PipelineKey,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlphaMode{
    Opaque,
    Mask,
    Blend,
}

impl Material {
    /// Resolves texture assets for rendering without changing ownership counts.
    pub fn to_render_material(&self, resources: &Resources) -> Result<RenderMaterial> {
        let resolve = |id: Option<TextureAssetId>| {
            id.map(|id| {
                resources.get_texture_handle(id)
                    .ok_or_else(|| anyhow!("texture asset not found: {id:?}"))
            }).transpose()
        };

        Ok(RenderMaterial {
            base_color: self.base_color,
            base_color_texture: resolve(self.base_color_texture)?,
            metallic: self.metallic,
            roughness: self.roughness,
            metallic_roughness_texture: resolve(self.metallic_roughness_texture)?,
            normal_texture: resolve(self.normal_texture)?,
            normal_scale: self.normal_scale,
            emissive_color: self.emissive_color,
            emissive_strength: self.emissive_strength,
            emissive_texture: resolve(self.emissive_texture)?,
            occlusion_strength: self.occlusion_strength,
            occlusion_texture: resolve(self.occlusion_texture)?,
            alpha_mode: match self.alpha_mode {
                AlphaMode::Opaque => RenderAlphaMode::Opaque,
                AlphaMode::Mask => RenderAlphaMode::Mask,
                AlphaMode::Blend => RenderAlphaMode::Blend,
            },
            alpha_cutoff: self.alpha_cutoff,
            double_sided: self.double_sided,
            pipeline_key: self.pipeline_key,
        })
    }

    pub const fn new(
        base_color: Vector4<f32>,
        base_color_texture: Option<TextureAssetId>,
        metallic: f32,
        roughness: f32,
        metallic_roughness_texture: Option<TextureAssetId>,
        normal_texture: Option<TextureAssetId>,
        normal_scale :f32,
        emissive_color: Vector3<f32>,
        emissive_strength: f32,
        emissive_texture: Option<TextureAssetId>,
        occlusion_strength: f32,
        occlusion_texture: Option<TextureAssetId>,
        alpha_mode: AlphaMode,
        alpha_cutoff: f32,
        double_sided: bool,
        pipeline_key: PipelineKey,
    ) -> Self {
        Self {
            base_color,
            base_color_texture,
            metallic,
            roughness,
            metallic_roughness_texture,
            normal_texture,
            normal_scale,
            emissive_strength,
            emissive_color,
            emissive_texture,
            occlusion_strength,
            occlusion_texture,
            alpha_mode,
            alpha_cutoff,
            double_sided,
            pipeline_key
        }
    }
}




impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: vec4(1.0,1.0,1.0,1.0),
            base_color_texture: None,

            metallic: 0.0,
            roughness: 0.0,
            metallic_roughness_texture: None,

            normal_texture: None,
            normal_scale: 1.0,

            emissive_color: vec3(0.0,0.0,0.0),
            emissive_strength: 1.0,
            emissive_texture: None,

            occlusion_strength: 1.0,
            occlusion_texture: None,

            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,

            double_sided: false,

            pipeline_key: PipelineKey::Lit3D,
        }
    }
}

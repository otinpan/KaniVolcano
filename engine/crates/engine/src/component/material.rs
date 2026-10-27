use cgmath::{Vector3, Vector4, vec3, vec4};
use renderer_vulkan::{PipelineKey, TextureHandle};

use super::Component;
pub type Vec3 = Vector3<f32>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Material {
    pub base_color: Vector4<f32>,
    pub base_color_texture: Option<TextureHandle>,

    pub metallic: f32,

    pub roughness: f32,

    pub metallic_roughness_texture: Option<TextureHandle>,

    pub normal_texture: Option<TextureHandle>,
    pub normal_scale: f32, 

    pub emissive_color: Vector3<f32>,
    pub emissive_strength: f32,
    pub emissive_texture: Option<TextureHandle>,

    pub occlusion_strength: f32,

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
    pub const fn new(
        base_color: Vector4<f32>,
        base_color_texture: Option<TextureHandle>,
        metallic: f32,
        roughness: f32,
        metallic_roughness_texture: Option<TextureHandle>,
        normal_texture: Option<TextureHandle>,
        normal_scale :f32,
        emissive_color: Vector3<f32>,
        emissive_strength: f32,
        emissive_texture: Option<TextureHandle>,
        occlusion_strength: f32,
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
            alpha_mode,
            alpha_cutoff,
            double_sided,
            pipeline_key
        }
    }
}

impl Component for Material {}

impl From<&Material> for renderer_vulkan::RenderMaterial {
    fn from(material: &Material) -> Self {
        Self {
            base_color: material.base_color,
            base_color_texture: material.base_color_texture,
            metallic: material.metallic,
            roughness: material.roughness,
            metallic_roughness_texture: material.metallic_roughness_texture,
            normal_texture: material.normal_texture,
            normal_scale: material.normal_scale,
            emissive_color: material.emissive_color,
            emissive_strength: material.emissive_strength,
            emissive_texture: material.emissive_texture,
            occlusion_strength: material.occlusion_strength,
            alpha_mode: match material.alpha_mode {
                AlphaMode::Opaque => renderer_vulkan::RenderAlphaMode::Opaque,
                AlphaMode::Mask => renderer_vulkan::RenderAlphaMode::Mask,
                AlphaMode::Blend => renderer_vulkan::RenderAlphaMode::Blend,
            },
            alpha_cutoff: material.alpha_cutoff,
            double_sided: material.double_sided,
            pipeline_key: material.pipeline_key,
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

            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,

            double_sided: false,

            pipeline_key: PipelineKey::Lit3D,
        }
    }
}

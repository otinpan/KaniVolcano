use std::collections::HashMap;

use anyhow::Result;
use cgmath::vec3;
use kani_volcano_animation::{GltfModel, ModelMaterial};
use renderer_vulkan::{MeshHandle, VulkanRenderer};
use crate::{GltfModelAssetId, Material, PipelineKey, Resources};
use crate::resources::GltfTextureAsset;

/// Uploads and registers a decoded model on the renderer's owning thread.
/// Both loading paths share registration and rollback here.
pub(crate) unsafe fn upload_and_register_gltf_model(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    name: &str,
    gltf_model: GltfModel,
    auto_release: bool,
) -> Result<GltfModelAssetId> {
    anyhow::ensure!(resources.gltf_model_asset_id(name).is_none(), "model already registered: {name}");
    // send mesh to gpu
    let mesh_handles: Vec<Vec<MeshHandle>>=renderer.load_meshes_from_gltf(&gltf_model.meshes)?;

    // load texture
    let texture_handles = match renderer.load_textures_from_gltf(
        &gltf_model.images,
        &gltf_model.textures,
        &gltf_model.materials,
    ) {
        Ok(handles) => handles,
        Err(error) => {
            let cleanup = release_meshes(renderer, &mesh_handles);
            return Err(match cleanup {
                Ok(()) => error.context("failed to load model textures"),
                Err(cleanup) => error.context(format!("failed to load model textures; mesh cleanup failed: {cleanup:#}")),
            });
        }
    };


    // Register each GPU image once, preserving glTF texture indices.
    let mut texture_ids = HashMap::new();
    let textures: Vec<GltfTextureAsset> = texture_handles.iter().map(|texture| {
        let mut register = |handle| {
            *texture_ids.entry(handle).or_insert_with(|| {
                // not use register_texture, because it do not needed to register name
                // textures used in gltf do not have name
                resources.insert_texture_asset(handle, auto_release)
            })
        };
        GltfTextureAsset {
            srgb: texture.srgb.map(&mut register),
            linear: texture.linear.map(&mut register),
        }
    }).collect();

    // convert material
    let materials: Vec<Material> = gltf_model.materials
        .iter()
        .map(|source| {
            let mut material = convert_gltf_material(source);
            // The loader validates references and returns handles in glTF texture order.
            material.base_color_texture = source.base_color_texture
                .as_ref()
                .and_then(|reference| textures[reference.texture].srgb);
            material.emissive_texture = source.emissive_texture
                .as_ref()
                .and_then(|reference| textures[reference.texture].srgb);
            material.normal_texture = source.normal_texture
                .as_ref()
                .and_then(|reference| textures[reference.texture].linear);
            material.metallic_roughness_texture = source.metallic_roughness_texture
                .as_ref()
                .and_then(|reference| textures[reference.texture].linear);
            material.occlusion_texture = source.occlusion_texture
                .as_ref()
                .filter(|reference| reference.tex_coord==0)
                .and_then(|reference| textures[reference.texture].linear);
            material
        })
        .collect();


    // register model
    match resources.register_gltf_model(name, gltf_model, mesh_handles.clone(), materials, textures, auto_release) {
        Ok(id) => Ok(id),
        Err(error) => {
            resources.rollback_unpublished_texture_assets(texture_ids.into_values());
            let mut failures = Vec::new();
            if let Err(cleanup) = release_meshes(renderer, &mesh_handles) {
                failures.push(format!("mesh cleanup: {cleanup:#}"));
            }
            if let Err(cleanup) = renderer.rollback_gltf_textures(&texture_handles) {
                failures.push(format!("texture cleanup: {cleanup:#}"));
            }
            Err(error.context(format!("failed to register model asset; cleanup errors: {failures:?}")))
        }
    }
}

unsafe fn release_meshes(renderer: &mut VulkanRenderer, mesh_handles: &[Vec<MeshHandle>]) -> Result<()> {
    let mut failures = Vec::new();
    for &mesh in mesh_handles.iter().flatten() {
        if let Err(error) = renderer.destroy_mesh(mesh) {
            failures.push(format!("{mesh:?}: {error:#}"));
        }
    }
    anyhow::ensure!(failures.is_empty(), "{}", failures.join("; "));
    Ok(())
}

/// Converts material factors. Texture handles must be resolved after GPU upload.
fn convert_gltf_material(model_material: &ModelMaterial) -> Material {
    let mut material = Material {
        base_color: cgmath::vec4(
            model_material.base_color[0],
            model_material.base_color[1],
            model_material.base_color[2],
            model_material.base_color[3],
        ),
        metallic: model_material.metallic,
        roughness: model_material.roughness,
        // Temporarily reduce the imported normal scale to 10% to soften exaggerated bumps
        // while glTF texture transforms and tangent generation are not fully supported.
        // This is a visual workaround, not an accurate reproduction of the original material.
        normal_scale: model_material.normal_scale * 0.1,
        emissive_color: vec3(
            model_material.emissive[0],
            model_material.emissive[1],
            model_material.emissive[2],
        ),
        emissive_strength: model_material.emissive_strength,
        occlusion_strength: model_material.occlusion_strength,
        alpha_mode: match model_material.alpha_mode {
            kani_volcano_animation::ModelAlphaMode::Opaque => crate::component::AlphaMode::Opaque,
            kani_volcano_animation::ModelAlphaMode::Mask => crate::component::AlphaMode::Mask,
            kani_volcano_animation::ModelAlphaMode::Blend => crate::component::AlphaMode::Blend,
        },
        alpha_cutoff: model_material.alpha_cutoff,
        double_sided: model_material.double_sided,
        pipeline_key: PipelineKey::Pbr3D,
        ..Material::default()
    };
    // Temporary visual approximation, not physical transmission. Keep glass visible
    // at full transmission; replace this when transmission rendering is supported.
    let transmission = model_material.transmission_factor.clamp(0.0, 1.0);
    if transmission > 0.0 {
        const FULL_TRANSMISSION_ALPHA: f32 = 0.2;
        material.base_color.w *= 1.0 - transmission * (1.0 - FULL_TRANSMISSION_ALPHA);
        material.alpha_mode = crate::component::AlphaMode::Blend;
    }
    material
}

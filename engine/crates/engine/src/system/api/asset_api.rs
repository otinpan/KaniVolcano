use crate::app::DEFAULT_SKYBOX_TEXTURE;
use crate::{
    MeshAsset, MeshAssetId, PrimitiveType, RenderCommandApi, Resources,
    FontAsset, FontAssetId, AudioAsset, AudioAssetId, AudioEmitterResource, 
    AudioEmitterId, AudioBusResource, AudioBusId, GltfModelAssetId, TextureAssetId, TextureAsset,
};
use anyhow::{Result, anyhow};
use renderer_vulkan::{SkyboxTextureHandle, VertexLayout};


pub trait AssetApi {
    fn resources(&self) -> &Resources;
    fn resources_mut(&mut self) -> &mut Resources;

    /// Unregisters an unused texture regardless of auto_release and queues GPU destruction.
    /// Missing or referenced assets return an error without changing ownership.
    fn unload_texture(&mut self, id: TextureAssetId) -> Result<()>
    where Self: RenderCommandApi,
    {
        let handle = self.resources_mut().unload_texture(id)?;
        self.render_commands_mut().destroy_texture(handle);
        Ok(())
    }

    /// Unregisters an unused mesh regardless of auto_release and queues GPU destruction.
    fn unload_mesh(&mut self, id: MeshAssetId) -> Result<()>
    where Self: RenderCommandApi,
    {
        let handle = self.resources_mut().unload_mesh(id)?;
        self.render_commands_mut().destroy_mesh(handle);
        Ok(())
    }

    /// Unregisters a model with no instances, regardless of its auto_release flag.
    /// Dependencies are released according to their own reference counts and auto_release flags.
    fn unload_gltf_model(&mut self, id: GltfModelAssetId) -> Result<()>
    where Self: RenderCommandApi,
    {
        let released = self.resources_mut().unload_gltf_model(id)?;
        let commands = self.render_commands_mut();
        for mesh in released.meshes {
            commands.destroy_mesh(mesh);
        }
        for texture in released.textures {
            commands.destroy_texture(texture);
        }
        Ok(())
    }

    fn model_asset_id(&self, model_name: &str) -> Result<MeshAssetId> {
        let asset_id = self
            .resources()
            .model_asset_id(model_name)
            .ok_or_else(|| anyhow!("model not found: {model_name}"))?;

        Ok(asset_id)
    }

    fn gltf_model_asset_id(&self, model_name: &str) -> Result<GltfModelAssetId>{
        let asset_id = self
            .resources()
            .gltf_model_asset_id(model_name)
            .ok_or_else(|| anyhow!("model not found: {model_name}"))?;

        Ok(asset_id)
    }

    /// Looks up a texture asset without retaining it. Use `None` in Material
    /// when no texture is needed.
    fn texture_asset_id(&self, texture_name: &str) -> Result<TextureAssetId> {
        self.resources()
            .texture_asset_id(texture_name)
            .ok_or_else(|| anyhow!("texture not found: {texture_name}"))
    }

    // return a MeshAssetId that match (primitive_type, vertex_layout)
    fn primitive_asset_id(
        &self,
        primitive_type: PrimitiveType,
        vertex_layout: VertexLayout,
    ) -> Option<MeshAssetId> {
        self.resources()
            .primitive_asset_id(primitive_type, vertex_layout)
    }

    fn set_skybox(&mut self, texture_name: &str) -> Result<()>
    where
        Self: RenderCommandApi,
    {
        let mesh = self
            .resources()
            .skybox_mesh()
            .ok_or_else(|| anyhow!("skybox mesh is not registered"))?;

        let texture = self
            .resources()
            .skybox_texture(texture_name)
            .unwrap_or(self.default_skybox_texture());

        self.render_commands_mut().set_skybox(mesh, texture);

        Ok(())
    }

    fn default_skybox_texture(&self) -> SkyboxTextureHandle {
        DEFAULT_SKYBOX_TEXTURE
    }

    fn primitive_type_from_asset_id(&self, asset_id: MeshAssetId) -> Option<PrimitiveType> {
        self.resources().primitive_type_from_asset_id(asset_id)
    }

    fn vertex_layout_from_asset_id(&self, asset_id: MeshAssetId) -> Option<VertexLayout> {
        self.resources().vertex_layout_from_asset_id(asset_id)
    }

    fn mesh_assets(&self) -> impl Iterator<Item = (MeshAssetId, &MeshAsset)> {
        self.resources().mesh_assets()
    }

    fn texture_assets(&self) -> impl Iterator<Item = (TextureAssetId, &TextureAsset)> {
        self.resources().texture_assets()
    }

    // font_asset
    fn font_assets(&self) -> impl Iterator<Item = (FontAssetId, &FontAsset)>{
        self.resources().font_assets()
    }

    fn font_asset_id(&self, name: &str) -> Result<FontAssetId> {
        self.resources()
            .font_asset_id(name)
            .ok_or_else(|| anyhow!("font not found: {name}"))
    }

    // audio
    fn audio_assets(&self) -> impl Iterator<Item = (AudioAssetId, &AudioAsset)>{
        self.resources().audio_assets()
    }
    fn audio_asset_id(&self, name: &str) -> Result<AudioAssetId>{
        self.resources()
            .audio_asset_id(name)
            .ok_or_else(|| anyhow!("audio not found: {name}"))
    }
    fn audio_emitters(&self) -> impl Iterator<Item = (AudioEmitterId, &AudioEmitterResource)>{
        self.resources().audio_emitters()
    }
    fn audio_emitter_id(&self, name: &str) -> Result<AudioEmitterId>{
        self.resources()
            .audio_emitter_id(name)
            .ok_or_else(|| anyhow!("audio emitter not found: {name}"))
    }
    fn audio_buses(&self) -> impl Iterator<Item = (AudioBusId, &AudioBusResource)>{
        self.resources().audio_buses()
    }
    fn audio_bus_id(&self, name: &str) -> Result<AudioBusId>{
        self.resources()
            .audio_bus_id(name)
            .ok_or_else(|| anyhow!("audio bus not found: {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RenderCommand, RenderCommandQueue};
    use renderer_vulkan::{MeshHandle, TextureHandle};

    #[derive(Default)]
    struct Context {
        resources: Resources,
        commands: RenderCommandQueue,
    }
    impl AssetApi for Context {
        fn resources(&self) -> &Resources { &self.resources }
        fn resources_mut(&mut self) -> &mut Resources { &mut self.resources }
    }
    impl RenderCommandApi for Context {
        fn render_commands_mut(&mut self) -> &mut RenderCommandQueue { &mut self.commands }
    }

    #[test]
    fn unload_requires_no_references_and_queues_destruction_once() {
        let mut context = Context::default();
        let mesh_handle = MeshHandle::new(3, VertexLayout::Lit3D);
        let mesh = context.resources.register_model("mesh", mesh_handle, false);
        let texture = context.resources.register_texture("texture", TextureHandle(1), false).unwrap();
        context.resources.retain_mesh(mesh);
        context.resources.retain_texture(texture);
        assert!(context.unload_mesh(mesh).is_err());
        assert!(context.unload_texture(texture).is_err());
        assert_eq!(context.model_asset_id("mesh").unwrap(), mesh);
        assert_eq!(context.texture_asset_id("texture").unwrap(), texture);
        assert_eq!(context.commands.drain().count(), 0);
        assert!(context.resources.release_mesh(mesh).is_none());
        assert!(context.resources.release_texture(texture).is_none());
        context.unload_mesh(mesh).unwrap();
        context.unload_texture(texture).unwrap();
        assert!(context.resources.get_mesh_handle(mesh).is_none());
        assert!(context.resources.get_texture_handle(texture).is_none());
        assert!(context.model_asset_id("mesh").is_err());
        assert!(context.texture_asset_id("texture").is_err());
        assert!(context.unload_mesh(mesh).is_err());
        assert!(context.unload_texture(texture).is_err());
        let commands: Vec<_> = context.commands.drain().collect();
        assert!(matches!(commands.as_slice(),
            [RenderCommand::DestroyMesh { mesh }, RenderCommand::DestroyTexture { texture }]
                if *mesh == mesh_handle && *texture == TextureHandle(1)));
    }

    #[test]
    fn unloading_model_respects_shared_and_persistent_dependencies() {
        for auto_release in [false, true] {
            for shared in [false, true] {
                let mut context = Context::default();
                let mesh = context.resources.register_model("mesh",
                    MeshHandle::new(3, VertexLayout::Lit3D), auto_release);
                let texture = context.resources.register_texture("texture", TextureHandle(1), auto_release).unwrap();
                let model = context.resources.insert_gltf_model_asset(
                    kani_volcano_animation::GltfModel::default(), vec![vec![mesh, mesh]], vec![],
                    vec![crate::resources::GltfTextureAsset { srgb: Some(texture), linear: Some(texture) }],
                    false,
                ).unwrap();
                context.resources.retain_gltf_model(model).unwrap();
                assert!(context.unload_gltf_model(model).is_err());
                assert!(context.unload_mesh(mesh).is_err());
                assert!(context.unload_texture(texture).is_err());
                assert_eq!(context.commands.drain().count(), 0);
                let released = context.resources.release_gltf_model(model);
                assert!(released.meshes.is_empty() && released.textures.is_empty());
                if shared {
                    context.resources.retain_mesh(mesh);
                    context.resources.retain_texture(texture);
                }
                context.unload_gltf_model(model).unwrap();
                assert!(context.resources.gltf_model_asset(model).is_none());
                assert!(context.unload_gltf_model(model).is_err());
                let destroyed = auto_release && !shared;
                assert_eq!(context.resources.get_mesh_handle(mesh).is_none(), destroyed);
                assert_eq!(context.resources.get_texture_handle(texture).is_none(), destroyed);
                assert_eq!(context.commands.drain().count(), if destroyed { 2 } else { 0 });
                if !destroyed {
                    assert_eq!(context.resources.mesh_assets().next().unwrap().1.ref_count, usize::from(shared));
                    assert_eq!(context.resources.texture_assets().next().unwrap().1.ref_count, usize::from(shared));
                }
            }
        }
    }
}
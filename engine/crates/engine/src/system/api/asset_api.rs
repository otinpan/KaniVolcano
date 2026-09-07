use crate::app::{DEFAULT_SKYBOX_TEXTURE, DEFAULT_TEXTURE};
use crate::{
    MeshAsset, MeshAssetId, PrimitiveType, RenderCommandApi, Resources,
    FontAsset, FontAssetId,
};
use anyhow::{Result, anyhow};
use renderer_vulkan::{SkyboxTextureHandle, TextureHandle, VertexLayout};

pub trait AssetApi {
    fn resources(&self) -> &Resources;
    fn resources_mut(&mut self) -> &mut Resources;

    fn model_asset_id(&self, model_name: &str) -> Result<MeshAssetId> {
        let asset_id = self
            .resources()
            .model_asset_id(model_name)
            .ok_or_else(|| anyhow!("model not found: {model_name}"))?;

        Ok(asset_id)
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

    fn texture(&self, texture_name: &str) -> Result<TextureHandle> {
        self.resources()
            .get_texture_handle(texture_name)
            .ok_or_else(|| anyhow!("texture not found: {texture_name}"))
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

    fn default_texture(&self) -> TextureHandle {
        DEFAULT_TEXTURE
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

    // font_asset
    fn font_assets(&self) -> impl Iterator<Item = (FontAssetId, &FontAsset)>{
        self.resources().font_assets()
    }

    fn font_asset_id(&self, name: &str) -> Result<FontAssetId> {
        self.resources()
            .font_asset_id(name)
            .ok_or_else(|| anyhow!("font not found: {name}"))
    }
}

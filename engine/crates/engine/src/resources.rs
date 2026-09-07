use crate::{PrimitiveMesh, PrimitiveType};
use anyhow::Result;
use cgmath::Vector3;
use renderer_vulkan::{
    MeshHandle, SkyboxTextureHandle, TextureHandle, VertexLayout, VulkanRenderer,
};
use std::collections::HashMap;
use kani_volcano_text::{LoadedFont, TextSystem};

pub type Vec3 = Vector3<f32>;

// handle the number of entities that use this Mesh
#[derive(Debug)]
pub struct MeshAsset {
    pub handle: MeshHandle,
    pub ref_count: usize,
    pub auto_release: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshAssetId(pub usize);

#[derive(Debug)]
pub struct FontAsset{
    pub font: LoadedFont,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontAssetId(pub usize);

pub struct Resources {
    mesh_assets: Vec<Option<MeshAsset>>,
    models: HashMap<String, MeshAssetId>,
    textures: HashMap<String, TextureHandle>,
    primitive_meshes: Vec<PrimitiveMesh>,
    skybox_mesh: Option<MeshHandle>,
    skybox_textures: HashMap<String, SkyboxTextureHandle>,

    font_assets: Vec<Option<FontAsset>>,
    fonts: HashMap<String, FontAssetId>,
    text_system: TextSystem,
}

impl Resources {
    /// Parses and registers font bytes under a unique asset name.
    pub fn register_font(&mut self, name: &str, data: Vec<u8>) -> Result<FontAssetId> {
        anyhow::ensure!(!self.fonts.contains_key(name), "font already registered: {name}");
        let font = self.text_system.load_font(data)?;
        let id = FontAssetId(self.font_assets.len());
        self.font_assets.push(Some(FontAsset { font }));
        self.fonts.insert(name.to_string(), id);
        Ok(id)
    }

    pub fn font_asset(&self, id: FontAssetId) -> Option<&FontAsset> {
        self.font_assets.get(id.0)?.as_ref()
    }

    pub fn insert_mesh_asset(&mut self, handle: MeshHandle, auto_release: bool) -> MeshAssetId {
        let id = MeshAssetId(self.mesh_assets.len());

        self.mesh_assets.push(Some(MeshAsset {
            handle,
            ref_count: 0,
            auto_release,
        }));

        id
    }

    pub fn register_model(
        &mut self,
        name: impl Into<String>,
        handle: MeshHandle,
        auto_release: bool,
    ) -> MeshAssetId {
        let id = self.insert_mesh_asset(handle, auto_release);

        self.models.insert(name.into(), id);
        id
    }

    pub fn register_texture(&mut self, name: &str, handle: TextureHandle) -> TextureHandle {
        self.textures.insert(name.to_string(), handle);
        handle
    }

    pub fn register_skybox_texture(
        &mut self,
        name: &str,
        handle: SkyboxTextureHandle,
    ) -> SkyboxTextureHandle {
        self.skybox_textures.insert(name.to_string(), handle);
        handle
    }

    pub(crate) fn set_textures(&mut self, textures: HashMap<String, TextureHandle>) {
        self.textures = textures;
    }

    pub(crate) fn register_primitive_mesh(&mut self, mesh: PrimitiveMesh) -> PrimitiveMesh {
        self.primitive_meshes.push(mesh);
        mesh
    }

    pub(crate) fn set_primitive_meshes(&mut self, primitive_meshes: Vec<PrimitiveMesh>) {
        self.primitive_meshes = primitive_meshes;
    }

    pub(crate) fn set_skybox_mesh(&mut self, mesh: MeshHandle) {
        self.skybox_mesh = Some(mesh);
    }

    pub(crate) fn skybox_mesh(&self) -> Option<MeshHandle> {
        self.skybox_mesh
    }

    pub(crate) fn set_skybox_textures(
        &mut self,
        skybox_textures: HashMap<String, SkyboxTextureHandle>,
    ) {
        self.skybox_textures = skybox_textures;
    }

    pub(crate) fn skybox_texture(&self, name: &str) -> Option<SkyboxTextureHandle> {
        self.skybox_textures.get(name).copied()
    }

    // get asset id
    pub fn model_asset_id(&self, name: &str) -> Option<MeshAssetId> {
        self.models.get(name).copied()
    }

    pub fn get_texture_handle(&self, name: &str) -> Option<TextureHandle> {
        self.textures.get(name).copied()
    }

    pub fn primitive_asset_id(
        &self,
        primitive_type: PrimitiveType,
        vertex_layout: VertexLayout,
    ) -> Option<MeshAssetId> {
        self.primitive_meshes
            .iter()
            .find(|mesh| {
                mesh.primitive_type == primitive_type
                    && mesh.vertex_layout == vertex_layout
                    && self.get_mesh_handle(mesh.asset_id).is_some()
            })
            .map(|mesh| mesh.asset_id)
    }

    pub fn get_mesh_handle(&self, id: MeshAssetId) -> Option<MeshHandle> {
        self.mesh_assets
            .get(id.0)?
            .as_ref()
            .map(|asset| asset.handle)
    }

    // query for primitives
    pub fn primitive_type_from_asset_id(&self, asset_id: MeshAssetId) -> Option<PrimitiveType> {
        self.primitive_meshes
            .iter()
            .find(|mesh| mesh.asset_id == asset_id)
            .map(|mesh| mesh.primitive_type)
    }

    pub fn vertex_layout_from_asset_id(&self, asset_id: MeshAssetId) -> Option<VertexLayout> {
        self.primitive_meshes
            .iter()
            .find(|mesh| mesh.asset_id == asset_id)
            .map(|mesh| mesh.vertex_layout)
    }

    pub fn mesh_assets(&self) -> impl Iterator<Item = (MeshAssetId, &MeshAsset)> {
        self.mesh_assets
            .iter()
            .enumerate()
            .filter_map(|(index, mesh_asset)| {
                mesh_asset
                    .as_ref()
                    .map(|mesh_asset| (MeshAssetId(index), mesh_asset))
            })
    }

    // font
    pub fn font_assets(&self) -> impl Iterator<Item=(FontAssetId, &FontAsset)>{
        self.font_assets
            .iter()
            .enumerate()
            .filter_map(|(index, font_asset)|{
                font_asset
                    .as_ref()
                    .map(|font_asset| (FontAssetId(index), font_asset))
            })
    }
    pub fn font_asset_id(&self, name: &str) -> Option<FontAssetId> {
        self.fonts.get(name).copied()
    }

    // reference counter
    pub fn retain_mesh(&mut self, id: MeshAssetId) -> Option<MeshHandle> {
        let asset = self.mesh_assets.get_mut(id.0)?.as_mut()?;
        asset.ref_count += 1;
        Some(asset.handle)
    }

    pub fn release_mesh(&mut self, id: MeshAssetId) -> Option<MeshHandle> {
        let asset = self.mesh_assets.get_mut(id.0)?.as_mut()?;

        if asset.ref_count > 0 {
            asset.ref_count -= 1;
        }

        if asset.ref_count == 0 && asset.auto_release {
            let handle = asset.handle;
            self.mesh_assets[id.0] = None;
            return Some(handle);
        }

        None
    }

    pub(crate) unsafe fn release_mesh_for_renderer(
        &mut self,
        id: MeshAssetId,
        renderer: &mut VulkanRenderer,
    ) -> Result<()> {
        if let Some(handle) = self.release_mesh(id) {
            log::debug!("Mesh asset released and ready to destroy: {handle:?}");
            renderer.destroy_mesh(handle)?;
            assert!(renderer.data.meshes[handle.index].is_none());
        }

        Ok(())
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            mesh_assets: Vec::new(),
            models: HashMap::new(),
            textures: HashMap::new(),
            primitive_meshes: Vec::new(),
            skybox_mesh: None,
            skybox_textures: HashMap::new(),

            font_assets: Vec::new(),
            fonts: HashMap::new(),
            text_system: TextSystem::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_font_registration_leaves_resources_unchanged() {
        let mut resources = Resources::default();
        assert!(resources.register_font("invalid", b"not a font".to_vec()).is_err());
        assert_eq!(resources.font_asset_id("invalid"), None);
        assert_eq!(resources.font_assets().count(), 0);
        assert!(resources.font_asset(FontAssetId(0)).is_none());
        assert!(resources.font_asset(FontAssetId(usize::MAX)).is_none());
    }

    fn mesh_handle(index: usize) -> MeshHandle {
        MeshHandle::new(index, VertexLayout::Mesh3D)
    }

    fn mesh_asset(resources: &Resources, id: MeshAssetId) -> &MeshAsset {
        resources.mesh_assets[id.0].as_ref().unwrap()
    }

    #[test]
    fn insert_mesh_asset_starts_ref_count_at_zero() {
        let mut resources = Resources::default();

        let id = resources.insert_mesh_asset(mesh_handle(0), false);

        assert_eq!(mesh_asset(&resources, id).ref_count, 0);
        assert_eq!(mesh_asset(&resources, id).handle, mesh_handle(0));
    }

    #[test]
    fn retain_mesh_increments_ref_count_and_returns_handle() {
        let mut resources = Resources::default();
        let id = resources.insert_mesh_asset(mesh_handle(0), false);

        assert_eq!(resources.retain_mesh(id), Some(mesh_handle(0)));
        assert_eq!(mesh_asset(&resources, id).ref_count, 1);

        assert_eq!(resources.retain_mesh(id), Some(mesh_handle(0)));
        assert_eq!(mesh_asset(&resources, id).ref_count, 2);
    }

    #[test]
    fn release_mesh_decrements_ref_count_without_auto_release() {
        let mut resources = Resources::default();
        let id = resources.insert_mesh_asset(mesh_handle(0), false);
        resources.retain_mesh(id);
        resources.retain_mesh(id);

        assert_eq!(resources.release_mesh(id), None);
        assert_eq!(mesh_asset(&resources, id).ref_count, 1);

        assert_eq!(resources.release_mesh(id), None);
        assert_eq!(mesh_asset(&resources, id).ref_count, 0);
        assert!(resources.mesh_assets[id.0].is_some());
    }

    #[test]
    fn release_mesh_removes_auto_release_asset_when_ref_count_reaches_zero() {
        let mut resources = Resources::default();
        let id = resources.insert_mesh_asset(mesh_handle(0), true);
        resources.retain_mesh(id);

        assert_eq!(resources.release_mesh(id), Some(mesh_handle(0)));
        assert!(resources.mesh_assets[id.0].is_none());
    }

    #[test]
    fn release_mesh_for_missing_asset_returns_none() {
        let mut resources = Resources::default();

        assert_eq!(resources.release_mesh(MeshAssetId(99)), None);
        assert_eq!(resources.retain_mesh(MeshAssetId(99)), None);
    }
}

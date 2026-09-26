use crate::{PrimitiveMesh, PrimitiveType};
use anyhow::Result;
use cgmath::Vector3;
use renderer_vulkan::{
    MeshHandle, SkyboxTextureHandle, TextureHandle, VertexLayout, VulkanRenderer,
    FontHandle,
};
use kani_volcano_audio::{
    AudioHandle, AudioBusHandle, AudioEmitterHandle,
};
use std::collections::HashMap;

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
    pub handle: FontHandle,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontAssetId(pub usize);


#[derive(Debug)]
pub struct AudioAsset{
    pub handle: AudioHandle,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioAssetId(pub usize);

#[derive(Debug)]
pub struct AudioEmitterResource{
    pub handle: AudioEmitterHandle,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioEmitterId(pub usize);

#[derive(Debug)]
pub struct AudioBusResource{
    pub handle: AudioBusHandle,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioBusId(pub usize);

pub struct Resources {
    mesh_assets: Vec<Option<MeshAsset>>,
    models: HashMap<String, MeshAssetId>,
    textures: HashMap<String, TextureHandle>,
    primitive_meshes: Vec<PrimitiveMesh>,
    skybox_mesh: Option<MeshHandle>,
    skybox_textures: HashMap<String, SkyboxTextureHandle>,
    // font
    font_assets: Vec<Option<FontAsset>>,
    fonts: HashMap<String, FontAssetId>,
    // audio
    audio_assets: Vec<Option<AudioAsset>>,
    audio: HashMap<String,AudioAssetId>,
    audio_emitter_resources: Vec<Option<AudioEmitterResource>>,
    audio_emitters: HashMap<String, AudioEmitterId>,
    audio_bus_resources: Vec<Option<AudioBusResource>>,
    audio_buses: HashMap<String, AudioBusId>,
}

impl Resources {
    /// Parses and registers font bytes under a unique asset name.
    pub fn register_font(&mut self, name: &str, handle: FontHandle) -> Result<FontAssetId> {
        anyhow::ensure!(!self.fonts.contains_key(name), "font already registered: {name}");
        let id=FontAssetId(self.font_assets.len());

        self.font_assets.push(Some(FontAsset{handle}));
        self.fonts.insert(name.to_string(),id);
        Ok(id)
    }

    // audio
    pub fn register_audio(&mut self, name: &str, handle: AudioHandle) -> Result<AudioAssetId>{
        anyhow::ensure!(!self.audio.contains_key(name), "audio already registered: {name}");
        let id=AudioAssetId(self.audio_assets.len());

        self.audio_assets.push(Some(AudioAsset{handle}));
        self.audio.insert(name.to_string(),id);
        Ok(id)
    }
    pub fn register_audio_emitter(&mut self, name: &str, handle: AudioEmitterHandle) -> Result<AudioEmitterId>{
        anyhow::ensure!(!self.audio_emitters.contains_key(name), "audio emitter already registered: {name}");
        let id=AudioEmitterId(self.audio_emitter_resources.len());

        self.audio_emitter_resources.push(Some(AudioEmitterResource{handle}));
        self.audio_emitters.insert(name.to_string(), id);
        Ok(id)
    }
    pub fn register_audio_bus(&mut self, name: &str, handle: AudioBusHandle) -> Result<AudioBusId>{
        anyhow::ensure!(!self.audio_buses.contains_key(name), "audio bus already registered: {name}");
        let id=AudioBusId(self.audio_bus_resources.len());

        self.audio_bus_resources.push(Some(AudioBusResource{handle}));
        self.audio_buses.insert(name.to_string(), id);
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

    pub fn get_font_handle(&self, id: FontAssetId) -> Option<FontHandle>{
        self.font_assets
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

    // audio
    pub fn audio_assets(&self) -> impl Iterator<Item=(AudioAssetId, &AudioAsset)>{
        self.audio_assets
            .iter()
            .enumerate()
            .filter_map(|(index,audio_asset)|{
                audio_asset
                    .as_ref()
                    .map(|audio_asset| (AudioAssetId(index), audio_asset))
            })
    }
    pub fn audio_asset_id(&self, name: &str) -> Option<AudioAssetId>{
        self.audio.get(name).copied()
    }

    pub fn audio_emitters(&self) -> impl Iterator<Item=(AudioEmitterId, &AudioEmitterResource)>{
        self.audio_emitter_resources
            .iter()
            .enumerate()
            .filter_map(|(index, audio_emitter_resource)|{
                audio_emitter_resource
                    .as_ref()
                    .map(|resource| (AudioEmitterId(index), resource))
            })
    }
    pub fn audio_emitter_id(&self, name: &str) -> Option<AudioEmitterId>{
        self.audio_emitters.get(name).copied()
    }

    pub fn get_audio_emitter_handle(&self, id: AudioEmitterId) -> Option<AudioEmitterHandle> {
        self.audio_emitter_resources.get(id.0)?.as_ref()
            .map(|resource| resource.handle)
    }

    pub fn audio_buses(&self) -> impl Iterator<Item=(AudioBusId, &AudioBusResource)>{
        self.audio_bus_resources
            .iter()
            .enumerate()
            .filter_map(|(index, audio_bus_resource)|{
                audio_bus_resource
                    .as_ref()
                    .map(|audio_bus_resource| (AudioBusId(index), audio_bus_resource))
            })
    }
    pub fn audio_bus_id(&self, name: &str) -> Option<AudioBusId>{
        self.audio_buses.get(name).copied()
    }

    pub fn get_audio_bus_handle(&self, id: AudioBusId) -> Option<AudioBusHandle> {
        self.audio_bus_resources.get(id.0)?.as_ref()
            .map(|resource| resource.handle)
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
            audio_assets: Vec::new(),
            audio: HashMap::new(),
            audio_emitter_resources: Vec::new(),
            audio_emitters: HashMap::new(),
            audio_bus_resources: Vec::new(),
            audio_buses: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderer_vulkan::{FontHandle};

    #[test]
    fn duplicate_font_registration_preserves_original() {
        let mut resources = Resources::default();
        let id = resources.register_font("font", FontHandle(9)).unwrap();
        assert!(resources.register_font("font", FontHandle(10)).is_err());
        assert_eq!(resources.font_asset_id("font"), Some(id));
        assert_eq!(resources.font_assets().count(), 1);
        assert_eq!(resources.font_asset(id).unwrap().handle, FontHandle(9));
        assert_eq!(resources.get_font_handle(id), Some(FontHandle(9)));
        assert_eq!(resources.font_asset_id("missing"), None);
        assert!(resources.font_asset(FontAssetId(usize::MAX)).is_none());
    }

    #[test]
    fn audio_emitter_registration_preserves_handles_and_rejects_duplicate_names() {
        let mut resources = Resources::default();
        assert_eq!(resources.audio_emitter_id("missing"), None);
        assert_eq!(resources.get_audio_emitter_handle(AudioEmitterId(0)), None);

        let first = resources.register_audio_emitter("player", AudioEmitterHandle(9)).unwrap();
        assert!(resources.register_audio_emitter("player", AudioEmitterHandle(99)).is_err());
        let second = resources.register_audio_emitter("enemy", AudioEmitterHandle(42)).unwrap();

        assert_ne!(first, second);
        assert_eq!(resources.audio_emitter_id("player"), Some(first));
        assert_eq!(resources.audio_emitter_id("enemy"), Some(second));
        assert_eq!(resources.get_audio_emitter_handle(first), Some(AudioEmitterHandle(9)));
        assert_eq!(resources.get_audio_emitter_handle(second), Some(AudioEmitterHandle(42)));
        assert_eq!(resources.get_audio_emitter_handle(AudioEmitterId(usize::MAX)), None);
        assert_eq!(resources.audio_emitters().map(|(id, resource)| (id, resource.handle)).collect::<Vec<_>>(),
            vec![(first, AudioEmitterHandle(9)), (second, AudioEmitterHandle(42))]);
    }

    #[test]
    fn audio_bus_registration_preserves_handles_and_rejects_duplicate_names() {
        let mut resources = Resources::default();
        assert_eq!(resources.audio_bus_id("missing"), None);
        assert_eq!(resources.get_audio_bus_handle(AudioBusId(0)), None);

        let first = resources.register_audio_bus("master", AudioBusHandle(0)).unwrap();
        assert!(resources.register_audio_bus("master", AudioBusHandle(99)).is_err());
        let second = resources.register_audio_bus("reverb", AudioBusHandle(42)).unwrap();

        assert_ne!(first, second);
        assert_eq!(resources.audio_bus_id("master"), Some(first));
        assert_eq!(resources.audio_bus_id("reverb"), Some(second));
        assert_eq!(resources.get_audio_bus_handle(first), Some(AudioBusHandle(0)));
        assert_eq!(resources.get_audio_bus_handle(second), Some(AudioBusHandle(42)));
        assert_eq!(resources.get_audio_bus_handle(AudioBusId(usize::MAX)), None);
        assert_eq!(resources.audio_buses().map(|(id, resource)| (id, resource.handle)).collect::<Vec<_>>(),
            vec![(first, AudioBusHandle(0)), (second, AudioBusHandle(42))]);
    }

    #[test]
    fn audio_resource_ids_follow_storage_when_name_entries_are_removed() {
        let mut resources = Resources::default();
        let old_emitter = resources.register_audio_emitter("old", AudioEmitterHandle(9)).unwrap();
        let old_bus = resources.register_audio_bus("old", AudioBusHandle(9)).unwrap();
        // Simulate removing a name while retaining the storage slot.
        resources.audio_emitters.remove("old");
        resources.audio_buses.remove("old");
        resources.audio_emitter_resources[old_emitter.0] = None;
        resources.audio_bus_resources[old_bus.0] = None;

        let emitter = resources.register_audio_emitter("new", AudioEmitterHandle(42)).unwrap();
        let bus = resources.register_audio_bus("new", AudioBusHandle(42)).unwrap();

        assert_ne!(emitter, old_emitter);
        assert_ne!(bus, old_bus);
        assert_eq!(resources.get_audio_emitter_handle(old_emitter), None);
        assert_eq!(resources.get_audio_bus_handle(old_bus), None);
        assert_eq!(resources.get_audio_emitter_handle(emitter), Some(AudioEmitterHandle(42)));
        assert_eq!(resources.get_audio_bus_handle(bus), Some(AudioBusHandle(42)));
        assert_eq!(resources.audio_emitters().map(|(id, _)| id).collect::<Vec<_>>(), vec![emitter]);
        assert_eq!(resources.audio_buses().map(|(id, _)| id).collect::<Vec<_>>(), vec![bus]);
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

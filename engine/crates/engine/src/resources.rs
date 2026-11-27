use crate::{PrimitiveMesh, Material,  PrimitiveType};
use anyhow::{anyhow, Result};
use cgmath::Vector3;
use renderer_vulkan::{
    MeshHandle, SkyboxTextureHandle, TextureHandle, VertexLayout,
    FontHandle,
};
use kani_volcano_audio::{
    AudioHandle, AudioBusHandle, AudioEmitterHandle, PlaybackHandle,
};
use kani_volcano_animation::{
    GltfModel,
};
use std::collections::{HashMap, HashSet};

pub type Vec3 = Vector3<f32>;

// References held by entities and model assets using this mesh.
#[derive(Debug)]
pub struct MeshAsset {
    pub handle: MeshHandle,
    pub ref_count: usize,
    pub auto_release: bool,
}


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshAssetId(pub usize);

#[derive(Debug)]
pub struct TextureAsset{
    pub handle: TextureHandle,
    pub ref_count: usize,
    pub auto_release: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureAssetId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GltfTextureAsset{
    pub srgb: Option<TextureAssetId>,
    pub linear: Option<TextureAssetId>,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPlaybackResource{
    Pending, 
    Active(PlaybackHandle),
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioPlaybackId(pub usize);

#[derive(Debug)]
pub struct GltfModelAsset{
    pub source: GltfModel,
    pub meshes: Vec<Vec<MeshAssetId>>,
    pub materials: Vec<Material>,
    pub textures: Vec<GltfTextureAsset>,
    pub ref_count: usize,
    pub auto_release: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GltfModelAssetId(pub usize);

pub use renderer_vulkan::GltfTextureHandle;

#[derive(Default)]
pub struct ReleasedGltfModelAssets{
    pub meshes: Vec<MeshHandle>,
    pub textures: Vec<TextureHandle>,
}


pub struct Resources {
    mesh_assets: Vec<Option<MeshAsset>>,
    models: HashMap<String, MeshAssetId>,
    texture_assets: Vec<Option<TextureAsset>>,
    textures: HashMap<String, TextureAssetId>,
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
    audio_playback_resources: HashMap<AudioPlaybackId, AudioPlaybackResource>,
    next_audio_playback_id: usize,
    // gltf model
    gltf_model_assets: Vec<Option<GltfModelAsset>>,
    gltf_models: HashMap<String, GltfModelAssetId>,
}

impl Resources {
    pub(crate) fn remove_audio_emitter(&mut self, id: AudioEmitterId) {
        if let Some(slot) = self.audio_emitter_resources.get_mut(id.0) {
            *slot = None;
        }
        self.audio_emitters.retain(|_, value| *value != id);
    }

    pub(crate) fn remove_audio_bus(&mut self, id: AudioBusId) {
        if let Some(slot) = self.audio_bus_resources.get_mut(id.0) {
            *slot = None;
        }
        self.audio_buses.retain(|_, value| *value != id);
    }
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
    // reserves an ID before the queued playback is started.
    // Reserve a playback ID before processing the queue so callers can reference this specific playback immediately.
    // Unlike buses and emitters, playbacks are identified by generated IDs rather than user-provided names.
    pub fn reserve_audio_playback(&mut self) -> Result<AudioPlaybackId> {
        let id = AudioPlaybackId(self.next_audio_playback_id);
        let next = self.next_audio_playback_id.checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("audio playback ID exhausted"))?;
        self.audio_playback_resources.insert(id, AudioPlaybackResource::Pending);
        self.next_audio_playback_id = next;
        Ok(id)
    }

    pub fn activate_audio_playback(
        &mut self,
        id: AudioPlaybackId,
        handle: PlaybackHandle,
    ) -> Result<()> {
        let resource = self.audio_playback_resources.get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("audio playback not found: {id:?}"))?;
        anyhow::ensure!(matches!(resource, AudioPlaybackResource::Pending),
            "audio playback already active: {id:?}");
        *resource = AudioPlaybackResource::Active(handle);
        Ok(())
    }

    pub fn audio_playback(&self, id: AudioPlaybackId) -> Option<&AudioPlaybackResource> {
        self.audio_playback_resources.get(&id)
    }

    pub fn get_audio_playback_handle(&self, id: AudioPlaybackId) -> Option<PlaybackHandle> {
        match self.audio_playback(id)? {
            AudioPlaybackResource::Pending => None,
            AudioPlaybackResource::Active(handle) => Some(*handle),
        }
    }

    /// Removes the mapping only; this does not stop backend playback.
    pub fn remove_audio_playback(&mut self, id: AudioPlaybackId) -> Option<AudioPlaybackResource> {
        self.audio_playback_resources.remove(&id)
    }

    /// Removes mappings for handles returned by AudioSystem::collect_finished.
    pub fn remove_finished_audio_playbacks(&mut self, finished: &[PlaybackHandle]) {
        if finished.is_empty() {
            return;
        }
        let finished: HashSet<_> = finished.iter().copied().collect();
        self.audio_playback_resources.retain(|_, resource| match resource {
            AudioPlaybackResource::Pending => true,
            AudioPlaybackResource::Active(handle) => !finished.contains(handle),
        });
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

    pub fn insert_texture_asset(&mut self, handle: TextureHandle, auto_release: bool) -> TextureAssetId{
        let id = TextureAssetId(self.texture_assets.len());

        self.texture_assets.push(Some(TextureAsset{
            handle,
            ref_count: 0,
            auto_release,
        }));

        id
    }

    // Only for unnamed, unretained assets from a failed load. GPU cleanup is separate.
    pub(crate) fn rollback_unpublished_texture_assets(
        &mut self,
        ids: impl Iterator<Item = TextureAssetId>,
    ) {
        for id in ids {
            let slot = &mut self.texture_assets[id.0];
            assert_eq!(slot.as_ref().expect("missing texture asset").ref_count, 0);
            *slot = None;
        }
    }

    // Increment mesh and texture ownership counts.
    pub fn insert_gltf_model_asset(
        &mut self,
        gltf_model: GltfModel,
        meshes: Vec<Vec<MeshAssetId>>,
        materials: Vec<Material>,
        textures: Vec<GltfTextureAsset>,
        auto_release: bool,
    ) -> Result<GltfModelAssetId>{
        // One ownership reference per distinct mesh, even when several
        // primitives refer to the same asset. Validate before changing counts.
        let owned: HashSet<_> = meshes.iter().flatten().copied().collect();
        for mesh in &owned {
            let asset = self.mesh_assets.get(mesh.0).and_then(Option::as_ref)
                .ok_or_else(|| anyhow::anyhow!("mesh asset not found: {mesh:?}"))?;
            anyhow::ensure!(asset.ref_count < usize::MAX, "mesh reference count exhausted: {mesh:?}");
        }
        let owned_textures: HashSet<_> = textures.iter()
            .flat_map(|texture| [texture.srgb, texture.linear])
            .flatten().collect();
        for texture in &owned_textures {
            let asset = self.texture_assets.get(texture.0).and_then(Option::as_ref)
                .ok_or_else(|| anyhow!("texture asset not found: {texture:?}"))?;
            anyhow::ensure!(asset.ref_count < usize::MAX, "texture reference count exhausted: {texture:?}");
        }
        for mesh in owned {
            // increment mesh reference count
            self.retain_mesh(mesh);
        }
        for texture in owned_textures {
            self.retain_texture(texture);
        }

        // insert gltf model
        let id: GltfModelAssetId=GltfModelAssetId(self.gltf_model_assets.len());

        self.gltf_model_assets.push(Some(GltfModelAsset{
            source: gltf_model,
            meshes,
            materials,
            textures,
            ref_count: 0,
            auto_release,
        }));

        Ok(id)
    }

    /// Remove the model's ownership references. The caller must destroy every
    /// returned GPU handle; assets still retained by entities/models stay alive.
    #[must_use = "returned mesh and texture handles must be destroyed by the renderer"]
    pub fn remove_gltf_model_asset(&mut self, id: GltfModelAssetId) -> ReleasedGltfModelAssets {
        let Some(model) = self.gltf_model_assets.get_mut(id.0).and_then(Option::take) else {
            return ReleasedGltfModelAssets::default();
        };
        self.gltf_models.retain(|_, value| *value != id);
        let mut released = ReleasedGltfModelAssets::default();
        let mut seen = HashSet::new();
        for mesh in model.meshes.into_iter().flatten() {
            if seen.insert(mesh) {
                if let Some(handle) = self.release_mesh(mesh) {
                    released.meshes.push(handle);
                }
            }
        }
        let mut seen_textures = HashSet::new();
        for texture in model.textures.into_iter()
            .flat_map(|texture| [texture.srgb, texture.linear]).flatten()
        {
            if seen_textures.insert(texture) {
                if let Some(handle) = self.release_texture(texture) {
                    released.textures.push(handle);
                }
            }
        }
        released
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

    pub fn register_texture(
        &mut self,
        name: impl Into<String>,
        handle: TextureHandle,
        auto_release: bool,
    ) -> Result<TextureAssetId> {
        let name = name.into();
        anyhow::ensure!(!self.textures.contains_key(&name), "texture already registered: {name}");
        let id = self.insert_texture_asset(handle, auto_release);

        self.textures.insert(name, id);
        Ok(id)
    }

    pub fn register_gltf_model(
        &mut self,
        name: impl Into<String>,
        gltf_model: GltfModel,
        mesh_handles: Vec<Vec<MeshHandle>>,
        materials: Vec<Material>,
        textures: Vec<GltfTextureAsset>,
        auto_release: bool,
    ) -> Result<GltfModelAssetId>{
        let name = name.into();
        // Overwriting the name would strand the old model's ownership refs.
        // On rejection, the caller still owns the supplied GPU handles.
        anyhow::ensure!(!self.gltf_models.contains_key(&name), "model already registered: {name}");
        let first_mesh = self.mesh_assets.len();
        let mesh_assets = mesh_handles.into_iter().map(|handles| {
            handles.into_iter().map(|handle| self.insert_mesh_asset(handle, auto_release)).collect()
        }).collect();

        let result=self.insert_gltf_model_asset(
            gltf_model,
            mesh_assets,
            materials,
            textures,
            auto_release
        );
        let id = match result {
            Ok(id) => id,
            Err(error) => {
                self.mesh_assets.truncate(first_mesh);
                return Err(error);
            }
        };
        self.gltf_models.insert(name, id);
        Ok(id)
    }


    pub fn register_skybox_texture(
        &mut self,
        name: &str,
        handle: SkyboxTextureHandle,
    ) -> SkyboxTextureHandle {
        self.skybox_textures.insert(name.to_string(), handle);
        handle
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

    pub fn texture_asset_id(&self, name: &str) -> Option<TextureAssetId>{
        self.textures.get(name).copied()
    }

    pub fn gltf_model_asset_id(&self, name: &str) -> Option<GltfModelAssetId>{
        self.gltf_models.get(name).copied()
    }

    pub(crate) fn gltf_model_asset(&self, id: GltfModelAssetId) -> Option<&GltfModelAsset> {
        self.gltf_model_assets.get(id.0)?.as_ref()
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

    pub fn get_texture_handle(&self, id: TextureAssetId) -> Option<TextureHandle>{
        self.texture_assets
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

    pub fn texture_assets(&self) -> impl Iterator<Item=(TextureAssetId, &TextureAsset)>{
        self.texture_assets
            .iter()
            .enumerate()
            .filter_map(|(index, texture_asset) | {
                texture_asset
                    .as_ref()
                    .map(|texture_asset| (TextureAssetId(index), texture_asset))
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
    pub fn get_audio_handle(&self, id: AudioAssetId) -> Option<AudioHandle>{
        self.audio_assets.get(id.0)?.as_ref()
            .map(|resource| resource.handle)
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

    pub fn retain_texture(&mut self, id: TextureAssetId) -> Option<TextureHandle> {
        let asset = self.texture_assets.get_mut(id.0)?.as_mut()?;
        asset.ref_count += 1;
        Some(asset.handle)
    }

    fn material_texture_ids(material: &Material) -> [Option<TextureAssetId>; 5] {
        let mut ids = [
            material.base_color_texture,
            material.metallic_roughness_texture,
            material.normal_texture,
            material.emissive_texture,
            material.occlusion_texture,
        ];
        for i in 1..ids.len() {
            if ids[..i].contains(&ids[i]) {
                ids[i] = None;
            }
        }
        ids
    }

    /// Retains one reference per distinct texture. Errors leave all counts unchanged.
    pub fn retain_material_textures(&mut self, material: &Material) -> Result<()> {
        let ids = Self::material_texture_ids(material);
        for id in ids.into_iter().flatten() {
            let asset = self.texture_assets.get(id.0).and_then(Option::as_ref)
                .ok_or_else(|| anyhow!("texture asset not found: {id:?}"))?;
            anyhow::ensure!(asset.ref_count < usize::MAX, "texture reference count exhausted: {id:?}");
        }
        for id in ids.into_iter().flatten() {
            self.retain_texture(id);
        }
        Ok(())
    }

    pub fn retain_gltf_model(&mut self, id: GltfModelAssetId) -> Result<()>{
        let model=self.gltf_model_assets
            .get_mut(id.0)
            .and_then(Option::as_mut)
            .ok_or_else(|| anyhow!("glTF model asset not found: {id:?}"))?;

        model.ref_count = model.ref_count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!(
                "glTF model reference count exhausted: {id:?}"
            ))?;

        Ok(())
    }

    /// Releases one instance reference and returns assets eligible for GPU destruction.
    #[must_use]
    pub fn release_gltf_model(&mut self, id: GltfModelAssetId) -> ReleasedGltfModelAssets {
        let Some(model) = self.gltf_model_assets.get_mut(id.0).and_then(Option::as_mut) else {
            return ReleasedGltfModelAssets::default();
        };
        if model.ref_count == 0 {
            return ReleasedGltfModelAssets::default();
        }
        model.ref_count -= 1;
        if model.ref_count == 0 && model.auto_release {
            return self.remove_gltf_model_asset(id);
        }
        ReleasedGltfModelAssets::default()
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

    /// Releases one ownership reference and returns the handle for GPU destruction.
    #[must_use = "returned texture handles must be destroyed by the renderer"]
    pub fn release_texture(&mut self, id: TextureAssetId) -> Option<TextureHandle> {
        let asset = self.texture_assets.get_mut(id.0)?.as_mut()?;

        if asset.ref_count == 0 {
            return None;
        }
        asset.ref_count -= 1;

        if asset.ref_count == 0 && asset.auto_release {
            let handle = asset.handle;
            self.texture_assets[id.0] = None;
            self.textures.retain(|_, value| *value != id);
            return Some(handle);
        }

        None
    }

    /// Releases one reference per distinct texture; GPU destruction is left to the caller.
    #[must_use = "returned texture handles must be destroyed by the renderer"]
    pub fn release_material_textures(&mut self, material: &Material) -> Vec<TextureHandle> {
        Self::material_texture_ids(material).into_iter().flatten()
            .filter_map(|id| self.release_texture(id))
            .collect()
    }


}

impl Default for Resources {
    fn default() -> Self {
        Self {
            mesh_assets: Vec::new(),
            models: HashMap::new(),
            texture_assets: Vec::new(),
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
            audio_playback_resources: HashMap::new(),
            next_audio_playback_id: 0,
            gltf_model_assets: Vec::new(),
            gltf_models: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn material_textures_share_ownership_and_release_once() {
        let mut resources = Resources::default();
        let shared = resources.register_texture("shared", TextureHandle(7), true).unwrap();
        let persistent = resources.insert_texture_asset(TextureHandle(8), false);
        let material = Material {
            base_color_texture: Some(shared),
            metallic_roughness_texture: Some(shared),
            normal_texture: Some(persistent),
            emissive_texture: Some(shared),
            occlusion_texture: Some(persistent),
            ..Material::default()
        };
        resources.retain_material_textures(&material).unwrap();
        resources.retain_material_textures(&material).unwrap();
        assert_eq!(resources.texture_assets[shared.0].as_ref().unwrap().ref_count, 2);
        assert_eq!(resources.texture_assets[persistent.0].as_ref().unwrap().ref_count, 2);
        assert!(resources.release_material_textures(&material).is_empty());
        assert_eq!(resources.release_material_textures(&material), vec![TextureHandle(7)]);
        assert!(resources.get_texture_handle(shared).is_none());
        assert!(resources.texture_asset_id("shared").is_none());
        assert_eq!(resources.get_texture_handle(persistent), Some(TextureHandle(8)));
        assert_eq!(resources.texture_assets[persistent.0].as_ref().unwrap().ref_count, 0);
    }

    #[test]
    fn material_texture_retain_errors_do_not_change_counts() {
        let mut resources = Resources::default();
        let valid = resources.insert_texture_asset(TextureHandle(1), true);
        let exhausted = resources.insert_texture_asset(TextureHandle(2), true);
        resources.texture_assets[exhausted.0].as_mut().unwrap().ref_count = usize::MAX;
        for invalid in [TextureAssetId(99), exhausted] {
            let material = Material {
                base_color_texture: Some(valid),
                occlusion_texture: Some(invalid),
                ..Material::default()
            };
            assert!(resources.retain_material_textures(&material).is_err());
            assert_eq!(resources.texture_assets[valid.0].as_ref().unwrap().ref_count, 0);
            assert_eq!(resources.texture_assets[exhausted.0].as_ref().unwrap().ref_count, usize::MAX);
        }
    }

    #[test]
    fn material_without_textures_needs_no_ownership() {
        let mut resources = Resources::default();
        resources.retain_material_textures(&Material::default()).unwrap();
        assert!(resources.release_material_textures(&Material::default()).is_empty());
        assert!(resources.texture_assets.is_empty());
    }

    use super::*;
    use renderer_vulkan::{FontHandle};

    #[test]
    fn playback_reservation_activation_and_removal() {
        let mut resources = Resources::default();
        let id = resources.reserve_audio_playback().unwrap();
        assert_eq!(resources.audio_playback(id), Some(&AudioPlaybackResource::Pending));
        assert_eq!(resources.get_audio_playback_handle(id), None);
        resources.activate_audio_playback(id, PlaybackHandle(12)).unwrap();
        assert_eq!(resources.get_audio_playback_handle(id), Some(PlaybackHandle(12)));
        assert!(resources.activate_audio_playback(id, PlaybackHandle(99)).is_err());
        assert_eq!(resources.get_audio_playback_handle(id), Some(PlaybackHandle(12)));
        assert_eq!(resources.remove_audio_playback(id), Some(AudioPlaybackResource::Active(PlaybackHandle(12))));
        assert_eq!(resources.audio_playback(id), None);
        assert_eq!(resources.get_audio_playback_handle(id), None);
        assert_eq!(resources.remove_audio_playback(id), None);
        assert!(resources.activate_audio_playback(id, PlaybackHandle(99)).is_err());
        assert!(resources.activate_audio_playback(AudioPlaybackId(usize::MAX), PlaybackHandle(99)).is_err());
    }

    #[test]
    fn playback_ids_are_not_reused_after_removal() {
        let mut resources = Resources::default();
        let first = resources.reserve_audio_playback().unwrap();
        let second = resources.reserve_audio_playback().unwrap();
        assert_eq!(resources.remove_audio_playback(first), Some(AudioPlaybackResource::Pending));
        let third = resources.reserve_audio_playback().unwrap();
        assert_ne!(third, first);
        assert_ne!(third, second);
        assert_eq!(resources.audio_playback(second), Some(&AudioPlaybackResource::Pending));
    }

    #[test]
    fn playback_id_exhaustion_leaves_resources_unchanged() {
        let mut resources = Resources::default();
        let id = resources.reserve_audio_playback().unwrap();
        resources.next_audio_playback_id = usize::MAX;
        assert!(resources.reserve_audio_playback().is_err());
        assert_eq!(resources.next_audio_playback_id, usize::MAX);
        assert_eq!(resources.audio_playback_resources.len(), 1);
        assert_eq!(resources.audio_playback(id), Some(&AudioPlaybackResource::Pending));
    }

    #[test]
    fn finished_playback_cleanup_preserves_pending_and_other_active_playbacks() {
        let mut resources = Resources::default();
        let pending = resources.reserve_audio_playback().unwrap();
        let finished = resources.reserve_audio_playback().unwrap();
        let active = resources.reserve_audio_playback().unwrap();
        resources.activate_audio_playback(finished, PlaybackHandle(10)).unwrap();
        resources.activate_audio_playback(active, PlaybackHandle(20)).unwrap();
        resources.remove_finished_audio_playbacks(&[]);
        assert_eq!(resources.audio_playback_resources.len(), 3);
        resources.remove_finished_audio_playbacks(&[PlaybackHandle(10), PlaybackHandle(10), PlaybackHandle(99)]);
        assert_eq!(resources.audio_playback(finished), None);
        assert_eq!(resources.audio_playback(pending), Some(&AudioPlaybackResource::Pending));
        assert_eq!(resources.get_audio_playback_handle(active), Some(PlaybackHandle(20)));
        resources.remove_finished_audio_playbacks(&[PlaybackHandle(10)]);
        assert_eq!(resources.audio_playback_resources.len(), 2);
    }

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
        resources.remove_audio_emitter(old_emitter);
        resources.remove_audio_bus(old_bus);
        assert_eq!(resources.audio_emitter_id("old"), None);
        assert_eq!(resources.audio_bus_id("old"), None);

        let emitter = resources.register_audio_emitter("old", AudioEmitterHandle(42)).unwrap();
        let bus = resources.register_audio_bus("old", AudioBusHandle(42)).unwrap();
        resources.remove_audio_emitter(old_emitter);
        resources.remove_audio_bus(old_bus);
        assert_eq!(resources.audio_emitter_id("old"), Some(emitter));
        assert_eq!(resources.audio_bus_id("old"), Some(bus));

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

    fn empty_gltf_model() -> GltfModel {
        GltfModel::default()
    }

    #[test]
    fn model_keeps_mesh_alive_after_last_entity_and_allows_respawn() {
        let mut resources = Resources::default();
        let materials = vec![Material { metallic: 0.25, ..Material::default() }];
        let srgb = resources.insert_texture_asset(TextureHandle(7), true);
        let linear = resources.insert_texture_asset(TextureHandle(8), true);
        let textures = vec![
            GltfTextureAsset { srgb: Some(srgb), linear: None },
            GltfTextureAsset { srgb: None, linear: Some(linear) },
            GltfTextureAsset { srgb: Some(srgb), linear: None },
        ];
        let model = resources.register_gltf_model("character", empty_gltf_model(),
            vec![vec![mesh_handle(3)]], materials.clone(), textures.clone(), true).unwrap();
        assert_eq!(resources.gltf_model_assets[model.0].as_ref().unwrap().materials, materials);
        assert_eq!(resources.gltf_model_assets[model.0].as_ref().unwrap().textures, textures);
        assert_eq!(resources.get_texture_handle(srgb), Some(TextureHandle(7)));
        assert_eq!(resources.get_texture_handle(linear), Some(TextureHandle(8)));
        // Repeated glTF texture references contribute only one ownership reference.
        assert_eq!(resources.texture_assets[srgb.0].as_ref().unwrap().ref_count, 1);
        assert_eq!(resources.texture_assets[linear.0].as_ref().unwrap().ref_count, 1);
        let mesh = resources.gltf_model_assets[model.0].as_ref().unwrap().meshes[0][0];
        assert_eq!(mesh_asset(&resources, mesh).ref_count, 1);
        resources.retain_mesh(mesh);
        assert_eq!(resources.release_mesh(mesh), None);
        assert_eq!(resources.retain_mesh(mesh), Some(mesh_handle(3)));
        assert_eq!(resources.release_mesh(mesh), None);
        let released = resources.remove_gltf_model_asset(model);
        assert_eq!(released.meshes, vec![mesh_handle(3)]);
        assert_eq!(released.textures, vec![TextureHandle(7), TextureHandle(8)]);
        assert!(resources.get_texture_handle(srgb).is_none());
        assert!(resources.get_texture_handle(linear).is_none());
        assert!(resources.get_mesh_handle(mesh).is_none());
        assert!(!resources.gltf_models.contains_key("character"));
        let released = resources.remove_gltf_model_asset(model);
        assert!(released.meshes.is_empty() && released.textures.is_empty());
    }

    #[test]
    fn releasing_model_preserves_shared_and_persistent_textures() {
        for auto_release in [false, true] {
            let mut resources = Resources::default();
            let texture = resources.insert_texture_asset(TextureHandle(9), true);
            let persistent = resources.insert_texture_asset(TextureHandle(10), false);
            let model = resources.insert_gltf_model_asset(empty_gltf_model(), vec![], vec![],
                vec![GltfTextureAsset { srgb: Some(texture), linear: Some(texture) },
                     GltfTextureAsset { srgb: Some(persistent), linear: None }], auto_release).unwrap();
            resources.retain_texture(texture);
            resources.retain_gltf_model(model).unwrap();
            resources.retain_gltf_model(model).unwrap();
            assert!(resources.release_gltf_model(model).textures.is_empty());
            assert_eq!(resources.texture_assets[texture.0].as_ref().unwrap().ref_count, 2);
            let released = resources.release_gltf_model(model);
            assert!(released.meshes.is_empty() && released.textures.is_empty());
            if !auto_release {
                assert_eq!(resources.gltf_model_asset(model).unwrap().ref_count, 0);
                assert_eq!(resources.texture_assets[texture.0].as_ref().unwrap().ref_count, 2);
                let released = resources.remove_gltf_model_asset(model);
                assert!(released.meshes.is_empty() && released.textures.is_empty());
            }
            assert!(resources.gltf_model_asset(model).is_none());
            assert_eq!(resources.texture_assets[texture.0].as_ref().unwrap().ref_count, 1);
            assert_eq!(resources.texture_assets[persistent.0].as_ref().unwrap().ref_count, 0);
            assert_eq!(resources.release_texture(texture), Some(TextureHandle(9)));
            let released = resources.release_gltf_model(model);
            assert!(released.meshes.is_empty() && released.textures.is_empty());
        }
    }

    #[test]
    fn model_removal_preserves_entity_and_other_model_references() {
        let mut resources = Resources::default();
        let mesh = resources.insert_mesh_asset(mesh_handle(4), true);
        let first = resources.insert_gltf_model_asset(empty_gltf_model(),
            vec![vec![mesh]], Vec::new(), Vec::new(), true).unwrap();
        let second = resources.insert_gltf_model_asset(empty_gltf_model(), vec![vec![mesh]], Vec::new(), Vec::new(), true).unwrap();
        assert_eq!(mesh_asset(&resources, mesh).ref_count, 2);
        resources.retain_mesh(mesh);
        assert!(resources.remove_gltf_model_asset(first).meshes.is_empty());
        assert!(resources.remove_gltf_model_asset(second).meshes.is_empty());
        assert_eq!(mesh_asset(&resources, mesh).ref_count, 1);
        assert_eq!(resources.release_mesh(mesh), Some(mesh_handle(4)));
    }

    #[test]
    fn invalid_model_mesh_does_not_partially_retain_assets() {
        let mut resources = Resources::default();
        let mesh = resources.insert_mesh_asset(mesh_handle(5), true);
        assert!(resources.insert_gltf_model_asset(empty_gltf_model(),
            vec![vec![mesh, MeshAssetId(999)]], Vec::new(), Vec::new(), true).is_err());
        assert_eq!(mesh_asset(&resources, mesh).ref_count, 0);
        assert!(resources.gltf_model_assets.is_empty());
    }

    #[test]
    fn removing_model_respects_non_auto_release_meshes() {
        let mut resources = Resources::default();
        let mesh = resources.insert_mesh_asset(mesh_handle(6), false);
        let model = resources.insert_gltf_model_asset(empty_gltf_model(), vec![vec![mesh]], Vec::new(), Vec::new(), false).unwrap();
        assert!(resources.remove_gltf_model_asset(model).meshes.is_empty());
        assert_eq!(mesh_asset(&resources, mesh).ref_count, 0);
        assert_eq!(resources.get_mesh_handle(mesh), Some(mesh_handle(6)));
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

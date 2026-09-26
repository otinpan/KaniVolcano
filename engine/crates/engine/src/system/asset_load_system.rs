use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use anyhow::{anyhow, Context, Result};
use renderer_vulkan::{
    decode_skybox_texture, decode_texture, load_model_source, DebugLineVertex,
    DecodedTexture, Lit3DVertex, Mesh3DVertex, MeshData, VertexLayout, VulkanRenderer,
};
use kani_volcano_audio::{
    DecodedAudio, AudioSystem
};

use crate::Resources;
use super::{AssetLoadCommand, AssetLoadCommandQueue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind { Model, Texture, SkyboxTexture, Font, Audio }

/// One result per request, including failures. No GPU objects cross threads.
pub struct AssetLoadOutcome {
    pub kind: AssetKind,
    pub name: String,
    pub result: Result<()>,
}

enum PreparedModel {
    Mesh3D(MeshData<Mesh3DVertex>),
    DebugLine(MeshData<DebugLineVertex>),
    Lit3D(MeshData<Lit3DVertex>),
}

enum PreparedAsset {
    Model(PreparedModel, bool),
    Texture(DecodedTexture),
    SkyboxTexture(DecodedTexture),
    Font(Vec<u8>),
    Audio(DecodedAudio),
}

struct CompletedAssetLoad {
    kind: AssetKind,
    name: String,
    result: Result<PreparedAsset>,
}

fn prepare_asset(command: AssetLoadCommand) -> CompletedAssetLoad {
    let (kind, name) = match &command {
        AssetLoadCommand::LoadModel { name, .. } => (AssetKind::Model, name.clone()),
        AssetLoadCommand::LoadTexture { name, .. } => (AssetKind::Texture, name.clone()),
        AssetLoadCommand::LoadSkyboxTexture { name, .. } => (AssetKind::SkyboxTexture, name.clone()),
        AssetLoadCommand::LoadFont { name, .. } => (AssetKind::Font, name.clone()),
        AssetLoadCommand::LoadAudio{name , ..} => (AssetKind::Audio, name.clone()),
    };
    // A malformed asset must not kill the worker and strand later requests.
    let result = std::panic::catch_unwind(|| -> Result<PreparedAsset> {
        match command {
            AssetLoadCommand::LoadModel { path, pipeline_key, auto_release, .. } => {
                let layout = pipeline_key.required_vertex_layout();
                anyhow::ensure!(matches!(layout, VertexLayout::Mesh3D | VertexLayout::DebugLine3D | VertexLayout::Lit3D),
                    "unsupported model vertex layout: {:?}", layout);
                let source = load_model_source(&path).with_context(|| format!("model: {path}"))?;
                let model = match layout {
                    VertexLayout::Mesh3D => PreparedModel::Mesh3D(source.to_mesh3d_data()),
                    VertexLayout::DebugLine3D => PreparedModel::DebugLine(source.to_debugline_data()),
                    VertexLayout::Lit3D => PreparedModel::Lit3D(source.to_lit3d_data()),
                    _ => unreachable!(),
                };
                Ok(PreparedAsset::Model(model, auto_release))
            }
            AssetLoadCommand::LoadTexture { path, .. } => {
                Ok(PreparedAsset::Texture(decode_texture(&path).with_context(|| format!("texture: {path}"))?))
            }
            AssetLoadCommand::LoadSkyboxTexture { paths, .. } => {
                Ok(PreparedAsset::SkyboxTexture(decode_skybox_texture(std::array::from_fn(|i| paths[i].as_str()))?))
            }
            AssetLoadCommand::LoadFont { path, .. } => {
                Ok(PreparedAsset::Font(std::fs::read(&path).with_context(|| format!("font: {path}"))?))
            }
            AssetLoadCommand::LoadAudio{path, ..} =>{
                Ok(PreparedAsset::Audio(DecodedAudio::from_file(&path)?))
            }
        }
    }).unwrap_or_else(|_| Err(anyhow!("asset preparation panicked")));
    CompletedAssetLoad { kind, name, result }
}

pub struct AssetLoadSystem {
    request_tx: Option<Sender<AssetLoadCommand>>,
    completed_rx: Option<Receiver<CompletedAssetLoad>>,
    worker: Option<JoinHandle<()>>,
}
// new() -> create worker thread
// -> load_asset_stage() -> queued in request_tx(AssetLoadCommand) -> request worker thread 
// -> load asset(file read, create pixels...) -> CompletedAssetLoad -> completed_rx
// -> register_asset_stage() -> register assets in resources and renderer
impl AssetLoadSystem {
    pub fn new() -> Result<Self> {
        // main thread to worker thread
        let (request_tx, request_rx) = mpsc::channel();
        // worker thread to main thread
        let (completed_tx, completed_rx) = mpsc::sync_channel(4);
        // create worker thread
        let worker = thread::Builder::new()
            .name("asset-loader".into())
            .spawn(move || {
                // when request from main thread
                while let Ok(command) = request_rx.recv() {
                    // send CompletedAssetLoad to main thread
                    if completed_tx.send(prepare_asset(command)).is_err() { break; }
                }
            })?;
        Ok(Self { request_tx: Some(request_tx), completed_rx: Some(completed_rx), worker: Some(worker) })
    }

    /// Dispatch only; never waits for disk I/O or decoding.
    pub fn load_asset_stage(&mut self, queue: &mut AssetLoadCommandQueue) -> Result<()> {
        let sender = self.request_tx.as_ref().ok_or_else(|| anyhow!("asset loader is shut down"))?;
        let mut disconnected = false;
        for command in queue.drain() {
            disconnected |= sender.send(command).is_err();
        }
        anyhow::ensure!(!disconnected, "asset worker disconnected; requests were not delivered");
        Ok(())
    }

    /// Call on the renderer's owning thread, before recording rendering commands.
    /// At most four completed assets are registered per call (not a time budget).
    pub unsafe fn register_asset_stage(
        &mut self, 
        renderer: &mut VulkanRenderer, 
        audio_system: &mut AudioSystem,
        resources: &mut Resources)
        -> Result<Vec<AssetLoadOutcome>>
    {
        let receiver = self.completed_rx.as_ref().ok_or_else(|| anyhow!("asset loader is shut down"))?;
        let mut outcomes = Vec::new();
        for _ in 0..4 {
            let completed = match receiver.try_recv() {
                Ok(value) => value,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) if !outcomes.is_empty() => break,
                Err(TryRecvError::Disconnected) => return Err(anyhow!("asset worker disconnected")),
            };
            let result = completed.result.and_then(|asset| unsafe {
                register_asset(renderer, audio_system, resources, &completed.name, asset)
            });
            outcomes.push(AssetLoadOutcome { kind: completed.kind, name: completed.name, result });
        }
        Ok(outcomes)
    }

    /// Discard queued results and stop after the current preparation finishes.
    /// File I/O already in progress cannot be interrupted.
    pub fn shutdown(&mut self) -> Result<()> {
        self.completed_rx.take();
        self.request_tx.take();
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| anyhow!("asset worker panicked"))?;
        }
        Ok(())
    }
}

impl Drop for AssetLoadSystem {
    fn drop(&mut self) { let _ = self.shutdown(); }
}

// register asseet to vulkan renderer and resources in main thread
unsafe fn register_asset(
    renderer: &mut VulkanRenderer,
    audio_system: &mut AudioSystem,
    resources: &mut Resources, 
    name: &str,
    asset: PreparedAsset) -> Result<()> 
{
    match asset {
        PreparedAsset::Model(model, auto_release) => {
            anyhow::ensure!(resources.model_asset_id(name).is_none(), "model already registered: {name}");
            let handle = match model {
                PreparedModel::Mesh3D(data) => renderer.load_mesh_from_data(data, VertexLayout::Mesh3D)?,
                PreparedModel::DebugLine(data) => renderer.load_mesh_from_data(data, VertexLayout::DebugLine3D)?,
                PreparedModel::Lit3D(data) => renderer.load_mesh_from_data(data, VertexLayout::Lit3D)?,
            };
            resources.register_model(name, handle, auto_release);
        }
        PreparedAsset::Texture(image) => {
            anyhow::ensure!(resources.get_texture_handle(name).is_none(), "texture already registered: {name}");
            let handle = renderer.upload_decoded_texture(&image)?;
            resources.register_texture(name, handle);
        }
        PreparedAsset::SkyboxTexture(image) => {
            anyhow::ensure!(resources.skybox_texture(name).is_none(), "skybox already registered: {name}");
            let handle = renderer.upload_decoded_skybox_texture(&image)?;
            resources.register_skybox_texture(name, handle);
        }
        PreparedAsset::Font(bytes) => {
            anyhow::ensure!(resources.font_asset_id(name).is_none(), "font already registered: {name}");
            let handle = renderer.load_font(bytes)?;
            resources.register_font(name, handle)?;
        }
        PreparedAsset::Audio(audio) =>{
            anyhow::ensure!(resources.audio_asset_id(name).is_none(), "audio already registerd: {name}");
            let handle=audio_system.register_audio(audio)?;
            resources.register_audio(name,handle)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderer_vulkan::PipelineKey;
    use std::time::Duration;

    fn fixture(path: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..").join(path).to_string_lossy().into_owned()
    }

    #[test]
    fn worker_reports_failure_then_continues_with_next_request() {
        let mut loader = AssetLoadSystem::new().unwrap();
        let mut queue = AssetLoadCommandQueue::default();
        queue.request_load_font("missing", &fixture("assets/no-such-font.ttf"));
        // Font preparation intentionally reads bytes; parsing belongs to registration.
        queue.request_load_font("bytes", &fixture("Cargo.toml"));
        loader.load_asset_stage(&mut queue).unwrap();
        assert_eq!(queue.drain().count(), 0);
        let receiver = loader.completed_rx.as_ref().unwrap();
        let failed = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(failed.name, "missing");
        assert!(failed.result.is_err());
        let loaded = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(loaded.kind, AssetKind::Font);
        match loaded.result.unwrap() {
            PreparedAsset::Font(bytes) => assert_eq!(bytes, std::fs::read(fixture("Cargo.toml")).unwrap()),
            _ => panic!("expected font bytes"),
        }
        loader.shutdown().unwrap();
        loader.shutdown().unwrap();
        assert!(loader.load_asset_stage(&mut queue).is_err());
    }

    #[test]
    fn model_preparation_supports_all_model_layouts() {
        for pipeline_key in [PipelineKey::Mesh3D, PipelineKey::DebugLine3D, PipelineKey::Lit3D] {
            let completed = prepare_asset(AssetLoadCommand::LoadModel {
                name: "model".into(), path: fixture("assets/models/viking_room.obj"),
                pipeline_key, auto_release: true,
            });
            match completed.result.unwrap() {
                PreparedAsset::Model(model, auto_release) => {
                    assert!(auto_release);
                    let (layout, indices) = match model {
                        PreparedModel::Mesh3D(data) => (VertexLayout::Mesh3D, data.indices),
                        PreparedModel::DebugLine(data) => (VertexLayout::DebugLine3D, data.indices),
                        PreparedModel::Lit3D(data) => (VertexLayout::Lit3D, data.indices),
                    };
                    assert_eq!(layout, pipeline_key.required_vertex_layout());
                    assert!(!indices.is_empty());
                }
                _ => panic!("expected model"),
            }
        }
        assert!(prepare_asset(AssetLoadCommand::LoadModel {
            name: "unsupported".into(), path: String::new(),
            pipeline_key: PipelineKey::Text3D, auto_release: false,
        }).result.is_err());
    }

    #[test]
    fn texture_and_cubemap_are_decoded_without_renderer() {
        let path = fixture("assets/textures/texture.png");
        let image = match prepare_asset(AssetLoadCommand::LoadTexture {
            name: "image".into(), path: path.clone(),
        }).result.unwrap() {
            PreparedAsset::Texture(image) => image,
            _ => panic!("expected texture"),
        };
        assert_eq!(image.pixels.len(), image.width as usize * image.height as usize * 4);
        let cube = prepare_asset(AssetLoadCommand::LoadSkyboxTexture {
            name: "cube".into(), paths: std::array::from_fn(|_| path.clone()),
        }).result;
        if image.width != image.height {
            assert!(cube.is_err());
        } else {
            match cube.unwrap() {
                PreparedAsset::SkyboxTexture(cube) => {
                    assert_eq!(cube.pixels.len(), image.pixels.len() * 6);
                    for face in cube.pixels.chunks_exact(image.pixels.len()) {
                        assert_eq!(face, image.pixels);
                    }
                }
                _ => panic!("expected cubemap"),
            }
        }
    }

    #[test]
    fn shutdown_does_not_wait_for_completed_results_to_be_consumed() {
        let (done_tx, done_rx) = mpsc::channel();
        let task = thread::spawn(move || {
            let mut loader = AssetLoadSystem::new().unwrap();
            let mut queue = AssetLoadCommandQueue::default();
            for _ in 0..12 {
                queue.request_load_font("bytes", &fixture("Cargo.toml"));
            }
            loader.load_asset_stage(&mut queue).unwrap();
            // Wait for one result, then stop without draining the rest.
            loader.completed_rx.as_ref().unwrap().recv_timeout(Duration::from_secs(5)).unwrap();
            loader.shutdown().unwrap();
            done_tx.send(()).unwrap();
        });
        done_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        task.join().unwrap();
    }
}

#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod app;
mod component;
mod ecs;
mod input;
mod primitive;
mod resources;
mod runner;
mod scene;
mod scene_manager;
mod system;
mod time;
mod world;

pub use app::App;
pub use component::{
    Camera, CameraComponent, Component, Material, MeshRenderer, Name, PendingPrimitiveMesh,
    Rotator, SceneId, SceneOwned, Tags, Visibility, Text,
};
pub use ecs::{ComponentPool, EntityId, Registry};
pub use input::Input;
pub use primitive::{PrimitiveMesh, PrimitiveShape, PrimitiveType};
pub use renderer_vulkan::{PipelineKey, VertexLayout};
pub use resources::{MeshAsset, MeshAssetId, Resources, FontAssetId, FontAsset};
pub use runner::{EngineConfig, run};
pub use scene::{Scene, SceneContext};
pub use scene_manager::SceneManager;
pub use system::{
    AssetApi, CameraSystem, Command, CommandContext, CommandQueue, CommandRef, CommandSystem,
    CreatePrimitiveCommand, DebugMonitor, DespawnLastCommand, EntityApi, InputApi, InputSystem,
    InputTrigger, KeyBinding, ObjectApi, RenderCommand, RenderCommandApi, RenderCommandQueue,
    RenderSystem, RotatorSystem, SceneCommand, SceneCommandApi, SceneCommandQueue, Scheduler,
    SpawnPrimitiveCommand, SpawnVikingRoomCommand, TimeApi, UpdateContext,
    UpdatePrimitiveMeshesCommand, UpdateSystem,
};
pub use time::Time;
pub use world::World;

pub mod prelude {
    pub use crate::{
        App, AssetApi, Camera, CameraSystem, Command, CommandContext, Component, EngineConfig,
        EntityApi, EntityId, InputApi, InputTrigger, Material, MeshAssetId, MeshRenderer, Name,
        ObjectApi, PipelineKey, PrimitiveShape, PrimitiveType, RenderCommandApi, Rotator,
        RotatorSystem, Scene, SceneCommandApi, SceneContext, SceneId, SceneOwned, Tags, TimeApi,
        UpdateContext, UpdateSystem, Visibility, run, Text,
    };

    pub use kani_volcano_math::Transform;
}

#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod api;
mod command;
mod input_system;
mod render_command;
mod render_system;
mod scene_command;
mod scheduler;
mod update;
mod text_render_system;
mod asset_load_system;
mod asset_load_command;

pub use api::{
    AssetApi, EntityApi, InputApi, ObjectApi, RenderCommandApi, SceneCommandApi, TimeApi,
};
pub use command::{
    Command, CommandContext, CommandSystem, CreatePrimitiveCommand, DebugMonitor,
    DespawnLastCommand, SpawnPrimitiveCommand, SpawnVikingRoomCommand,
    UpdatePrimitiveMeshesCommand,
};
pub use input_system::{CommandQueue, CommandRef, InputSystem, InputTrigger, KeyBinding};
pub use render_command::{RenderCommand, RenderCommandQueue};
pub use render_system::{RenderContext, RenderSystem};
pub use scene_command::{SceneCommand, SceneCommandQueue};
pub use scheduler::Scheduler;
pub use update::{CameraSystem, RotatorSystem, ScheduledUpdateSystem, UpdateContext, UpdateSystem};
pub use text_render_system::{TextRenderSystem, TextLayout};
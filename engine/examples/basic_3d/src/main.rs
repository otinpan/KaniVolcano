#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod basic_3d_scene;
mod basic_field_scene;
use anyhow::Result;
use basic_3d_scene::Basic3dScene;
pub use basic_field_scene::{BasicFieldScene, ChangeSceneCommand};
use kani_volcano_engine::prelude::*;
use kani_volcano_engine::{App, EngineConfig, run};

fn main() -> Result<()> {
    pretty_env_logger::init();

    run(
        EngineConfig {
            title: "Vulkan Tutorial (Rust)".to_string(),
            width: 1024,
            height: 768,
        },
        |app| {
            app.create_skybox(10.0)?;
            load_assets(app)?;
            app.add_scene(Basic3dScene::default())?;
            app.add_scene(BasicFieldScene::default())?;
            app.set_current_scene("Basic3dScene")?;
            Ok(())
        },
    )
}

fn load_assets(app: &mut App) -> Result<()> {
    // load models
    unsafe {
        load_models(app)?;
        load_textures(app)?;
        load_skybox_textures(app)?;
        load_fonts(app)?;
    }
    Ok(())
}

unsafe fn load_models(app: &mut App) -> Result<()> {
    app.load_model(
        "viking_room",
        "assets/models/viking_room.obj",
        PipelineKey::Mesh3D,
        false,
    )?;
    app.load_model(
        "viking_room_debug_line",
        "assets/models/viking_room.obj",
        PipelineKey::DebugLine3D,
        false,
    )?;
    app.load_model(
        "viking_room_lit3d",
        "assets/models/viking_room.obj",
        PipelineKey::Lit3D,
        false,
    )?;
    Ok(())
}

unsafe fn load_fonts(app: &mut App) -> Result<()>{
    app.load_font("eng_font", r"C:\Windows\Fonts\arial.ttf")?;
    app.load_font("jpn_font", r"C:\Windows\Fonts\NotoSansJP-VF.ttf")?;
    Ok(())
}

unsafe fn load_textures(app: &mut App) -> Result<()> {
    app.load_texture("viking_room", "assets/textures/viking_room.png")?;
    app.load_texture("face", "assets/textures/texture.png")?;
    app.load_texture("ghost", "assets/textures/ghost.png")?;
    app.load_texture("escapee", "assets/textures/escapee.png")?;

    Ok(())
}

unsafe fn load_skybox_textures(app: &mut App) -> Result<()> {
    app.load_skybox_texture(
        "escapee",
        [
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
        ],
    )?;
    app.load_skybox_texture(
        "ghost",
        [
            "assets/textures/ghost.png",
            "assets/textures/ghost.png",
            "assets/textures/ghost.png",
            "assets/textures/ghost.png",
            "assets/textures/ghost.png",
            "assets/textures/ghost.png",
        ],
    )?;

    Ok(())
}

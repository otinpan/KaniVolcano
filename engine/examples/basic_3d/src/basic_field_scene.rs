use anyhow::{Result, anyhow};
use cgmath::{vec3,vec2};
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;
use winit::keyboard::KeyCode;
pub struct BasicFieldScene {}

impl Scene for BasicFieldScene {
    fn name(&self) -> String {
        "BasicFieldScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.set_skybox("default")?;
        self.create_camera(context)?;
        self.create_fundation(context)?;
        self.create_texts(context)?;

        self.add_update_systems(context);

        self.bind_input_commands(context);
        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        Ok(())
    }
}

impl BasicFieldScene {
    fn create_camera(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let camera = context.spawn();
        context.add_component(camera, Transform::default());
        let success = context.add_component(
            camera,
            Camera {
                target: vec3(0.0, 0.0, 0.0),
                fov_y: 45.0,
                near: 0.1,
                far: 100.0,
                yaw: std::f32::consts::PI,
                pitch: 0.0,
            },
        );

        if !success {
            Err(anyhow!("failed to create Camera"))
        } else {
            Ok(())
        }
    }
    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("camera", CameraSystem);
    }

    fn create_fundation(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let foundation = context.spawn_rectangle_3d(
            vec3(0.0, 0.0, -1.0),
            30.0,
            30.0,
            vec3(0.0, -90.0, 0.0),
            vec3(0.5, 0.5, 0.5),
            1.0,
            None,
            PipelineKey::Mesh3D,
        )?;

        Ok(())
    }

    fn create_texts(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        let hello_world=context.spawn_text_3d(
            "eng_font",
            "Hello\nKaniVolcano",
            48.0,
            60.0,
            vec3(-3.0,0.0,0.0),
            vec3(0.01,0.01,0.01),
            vec3(45.0,45.0,0.0),
            vec3(0.0,1.0,0.0),
            0.5,
        );

        let ya=context.spawn_text_ui2d(
            "jpn_font",
            "やあ",
            100.0,
            100.0,
            vec2(0.5,0.5),
            vec2(1.0,2.0),
            45.0,
            vec3(1.0,0.0,1.0),
            1.0,
        );
        Ok(())
    }

    fn bind_input_commands(&mut self, context: &mut SceneContext<'_>) {
        context.bind_input_command(
            KeyCode::Space,
            InputTrigger::Pressed,
            ChangeSceneCommand {
                next_scene: "Basic3dScene".to_string(),
            },
        );
        context.bind_input_command(
            KeyCode::Digit0,
            InputTrigger::Pressed,
            LoadAssetsCommand,
        );
        context.bind_input_command(
            KeyCode::Enter,
            InputTrigger::Pressed,
            SpawnTransparentVikingRoom,
        );
    }
}

impl Default for BasicFieldScene {
    fn default() -> Self {
        Self {}
    }
}

pub struct ChangeSceneCommand {
    pub next_scene: String,
}

impl Command for ChangeSceneCommand {
    fn id(&self) -> String {
        format!("change_scene")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        context.set_current_scene(self.next_scene.as_str());
        Ok(())
    }
}

pub struct LoadAssetsCommand;

impl Command for LoadAssetsCommand{
    fn id(&self) -> String{
        format!("load_assets")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>{
        context.request_load_texture("jupiter","assets/textures/jupiter.png");
        context.request_load_model(
            "viking_room_lit3d",
            "assets/models/viking_room.obj",
            PipelineKey::Lit3D,
            true,
        );

        Ok(())
    }
}

pub struct SpawnTransparentVikingRoom;

impl Command for SpawnTransparentVikingRoom{
    fn id(&self) -> String{
        format!("spawn_transparent_viking_room")
    }

    fn execute(&self, context: &mut CommandContext) -> Result<()>{
        let jupiter=context.texture("jupiter")?;
        context.spawn_model(
            "viking_room_lit3d",
            Transform { 
                position: vec3(0.0,0.0,3.0), 
                rotation: vec3(0.0,45.0,0.0),
                scale: vec3(1.0,1.0,1.0)
            },
            Material { 
                color: vec3(1.0,1.0,1.0), 
                alpha: 0.5, 
                use_texture: true, 
                texture: jupiter,
                pipeline_key: PipelineKey::Lit3D
            }
        )?;
        Ok(())
    }
}
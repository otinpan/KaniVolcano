use anyhow::{Result, anyhow};
use cgmath::vec3;
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
        let font=context.font_asset_id("eng_font")?;

        let hello_world=context.spawn();
        context.add_component(
            hello_world,
            Transform {
                position: vec3(0.0, 0.0, 0.0),
                scale: vec3(1.0,2.0,0.0),
                ..Default::default()
            },
        );
        context.add_component(
            hello_world,
            Text{
                content: "Hello\nWorld".to_string(),
                font: font,
                font_size: 100.0,
                line_height: 100.0,
                color: vec3(1.0,1.0,0.0),
                alpha: 0.5,
            }
        );

        let wow=context.spawn_text(
            "jpn_font",
            "やあ",
            100.0,
            100.0,
            Transform{
                position: vec3(0.5,0.5,0.0),
                rotation: vec3(45.0,0.0,0.0),
                ..Default::default()
            },
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
        )
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

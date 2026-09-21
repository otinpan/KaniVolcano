use super::ChangeSceneCommand;
use anyhow::{Result, anyhow};
use cgmath::{Vector2, Vector3, vec2, vec3};
use kani_volcano_engine::prelude::*;
use kani_volcano_engine::{AssetApi, ObjectApi};
use kani_volcano_engine::{
    CameraSystem, CreatePrimitiveCommand, DebugMonitor, DespawnLastCommand, RotatorSystem, SceneId,
    SceneOwned, SpawnPrimitiveCommand, SpawnVikingRoomCommand, UpdatePrimitiveMeshesCommand,
};
use winit::keyboard::KeyCode;

type Material = kani_volcano_engine::Material;
type Transform = kani_volcano_math::Transform;
type Vec3 = Vector3<f32>;
type Vec2 = Vector2<f32>;

pub struct Basic3dScene {}

impl Scene for Basic3dScene {
    fn name(&self) -> String {
        "Basic3dScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.set_skybox("ghost")?;
        self.create_models(context)?;
        self.create_primitives(context)?;
        self.create_texts(context)?;
        self.create_camera(context)?;
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
impl Default for Basic3dScene {
    fn default() -> Self {
        Self {}
    }
}

impl Basic3dScene {
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
        context.add_update_system("rotator", RotatorSystem);
        context.add_update_system("camera", CameraSystem);
    }

    fn create_primitives(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let face_texture = context.texture("face").unwrap_or(context.default_texture());
        let ghost_texture = context
            .texture("ghost")
            .unwrap_or(context.default_texture());
        let escapee_texture = context
            .texture("escapee")
            .unwrap_or(context.default_texture());

        // primitives
        let triangle_id0 = context.spawn_triangle_3d(
            vec3(5.0, -0.2, -0.5),
            vec3(5.0, 0.5, 0.2),
            vec3(5.0, 0.0, 0.5),
            vec3(0.0, 1.0, 1.0),
            0.5,
            None,
            PipelineKey::Mesh3D,
        )?;
        let triangle_id1 = context.spawn_triangle_3d(
            vec3(0.0, -0.2, -0.5),
            vec3(0.0, 0.5, 0.2),
            vec3(0.0, 0.0, 0.5),
            vec3(0.0, 1.0, 1.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;
        let triangle_id2 = context.spawn_triangle_3d(
            vec3(-10.0, -0.2, -0.5),
            vec3(-10.0, 0.5, 0.2),
            vec3(-10.0, 0.0, 0.5),
            vec3(0.0, 1.0, 1.0),
            0.5,
            None,
            PipelineKey::DebugLine3D,
        )?;
        let triangle_id3 = context.spawn_triangle_3d(
            vec3(-5.0, -0.2, -0.5),
            vec3(-5.0, 0.5, 0.2),
            vec3(-5.0, 0.0, 0.5),
            vec3(0.0, 1.0, 1.0),
            1.0,
            Some("face"),
            PipelineKey::Lit3D,
        )?;
        let triangle_ui2d = context.spawn_triangle_2d(
            vec2(-0.6, 0.5),
            vec2(-0.7, 0.7),
            vec2(-0.8, 0.5),
            vec3(0.0, 1.0, 1.0),
            0.4,
            Some("face"),
        )?;

        let rectangle_id0 = context.spawn_rectangle_3d(
            vec3(5.0, 0.5, 0.5),
            0.3,
            0.3,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            0.5,
            None,
            PipelineKey::Mesh3D,
        )?;
        let rectangle_id1 = context.spawn_rectangle_3d(
            vec3(0.0, 0.5, 0.5),
            0.3,
            0.3,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;
        let rectangle_id2 = context.spawn_rectangle_3d(
            vec3(-10.0, 0.5, 0.5),
            0.3,
            0.3,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            0.5,
            None,
            PipelineKey::DebugLine3D,
        )?;
        let rectangle_id3 = context.spawn_rectangle_3d(
            vec3(-5.0, 0.5, 0.5),
            0.8,
            0.8,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            1.0,
            Some("escapee"),
            PipelineKey::Lit3D,
        )?;
        /*let rectangle_ui2d=app.spawn_rectangle_2d(
            vec2(0.0,0.3),
            0.5,
            0.3,
            45.0,
            vec3(1.0,0.0,1.0),
            1.0,
            Some(face_texture)
        )?;*/

        let cube_id0 = context.spawn_cube_3d(
            vec3(5.0, 1.0, 1.0),
            1.0,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::Mesh3D,
        )?;
        let cube_id1 = context.spawn_cube_3d(
            vec3(0.0, 1.0, 1.0),
            1.0,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;
        let cube_id2 = context.spawn_cube_3d(
            vec3(-10.0, 1.0, 1.0),
            1.0,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::DebugLine3D,
        )?;
        let cube_id3 = context.spawn_cube_3d(
            vec3(-5.0, 1.0, 1.0),
            1.0,
            vec3(0.0, 45.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            1.0,
            Some("face"),
            PipelineKey::Lit3D,
        )?;

        let cuboid_id0=context.spawn_cuboid_3d(
            vec3(5.0,-1.0,-1.0),
            0.5,
            1.0,
            4.0,
            vec3(0.0,0.0,0.0),
            vec3(1.0,1.0,0.0),
            0.5,
            None,
            PipelineKey::Mesh3D,
        )?;
        let cuboid_id1 = context.spawn_cuboid_3d(
            vec3(0.0, -1.0, -1.0),
                1.0,
            0.5,
            4.0,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;
        let cube_id2 = context.spawn_cuboid_3d(
            vec3(-10.0, -1.0, -1.0),
            4.0,
            0.5,
            1.0,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::DebugLine3D,
        )?;
        let cuboid_id3 = context.spawn_cuboid_3d(
            vec3(-5.0, -1.0, -1.0),
            1.0,
            4.0,
            0.5,
            vec3(0.0, 45.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            1.0,
            Some("face"),
            PipelineKey::Lit3D,
        )?;

        let circle_id0 = context.spawn_circle_3d(
            vec3(5.0, 2.0, 1.0),
            1.0,
            32,
            vec3(0.0, 0.0, 1.0),
            0.5,
            None,
            PipelineKey::Mesh3D,
        )?;
        let circle_id1 = context.spawn_circle_3d(
            vec3(0.0, 2.0, 1.0),
            1.0,
            32,
            vec3(0.0, 0.0, 1.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;
        let circle_id2 = context.spawn_circle_3d(
            vec3(-10.0, 2.0, 1.0),
            1.0,
            32,
            vec3(0.0, 0.0, 1.0),
            0.5,
            None,
            PipelineKey::DebugLine3D,
        )?;
        let circle_id3 = context.spawn_circle_3d(
            vec3(-5.0, 2.0, 1.0),
            1.0,
            32,
            vec3(0.0, 0.0, 1.0),
            1.0,
            Some("ghost"),
            PipelineKey::Lit3D,
        )?;
        /*let circle_ui2d=app.spawn_circle_2d(
            vec2(0.5,0.3),
            0.3,
            32,
            vec3(0.0,0.0,1.0),
            1.0,
            Some(face_texture),
        )?;*/

        let polygon_id0 = context.spawn_polygon_3d(
            vec![
                vec3(5.0, -0.4, -1.0),
                vec3(5.0, -0.2, 0.0),
                vec3(5.0, 0.5, -0.3),
                vec3(5.0, 0.3, 0.2),
                vec3(5.0, 0.0, 1.0),
                vec3(5.0, -0.1, 1.2),
            ],
            vec3(0.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::Mesh3D,
        )?;
        let polygon_id1 = context.spawn_polygon_3d(
            vec![
                vec3(0.0, -0.4, -1.0),
                vec3(0.0, -0.2, 0.0),
                vec3(0.0, 0.5, -0.3),
                vec3(0.0, 0.3, 0.2),
                vec3(0.0, 0.0, 1.0),
                vec3(0.0, -0.1, 1.2),
            ],
            vec3(0.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;
        let polygon_id2 = context.spawn_polygon_3d(
            vec![
                vec3(-10.0, -0.4, -1.0),
                vec3(-10.0, -0.2, 0.0),
                vec3(-10.0, 0.5, -0.3),
                vec3(-10.0, 0.3, 0.2),
                vec3(-10.0, 0.0, 1.0),
                vec3(-10.0, -0.1, 1.2),
            ],
            vec3(0.0, 1.0, 0.0),
            0.5,
            None,
            PipelineKey::DebugLine3D,
        )?;
        let polygon_id3 = context.spawn_polygon_3d(
            vec![
                vec3(-5.0, -0.4, -1.0),
                vec3(-5.0, -0.2, 0.0),
                vec3(-5.0, 0.5, -0.3),
                vec3(-5.0, 0.3, 0.2),
                vec3(-5.0, 0.0, 1.0),
                vec3(-5.0, -0.1, 1.2),
            ],
            vec3(0.0, 1.0, 0.0),
            1.0,
            Some("face"),
            PipelineKey::Lit3D,
        )?;
        /*let polygon_ui2d=app.spawn_polygon_2d(
            vec![
                vec2(-0.4, -1.0),
                vec2(-0.2, 0.0),
                vec2(0.5, -0.3),
                vec2(0.3, 0.2),
                vec2(0.0, 1.0),
                vec2(-0.1, 1.2),
            ],
            vec3(0.0,1.0,0.0),
            1.0,
            Some(face_texture),
        )?;*/

        let sphere_id0 = context.spawn_sphere_3d(
            vec3(5.0, -1.0, 0.0),
            0.5,
            16,
            16,
            vec3(1.0, 0.0, 0.0),
            0.5,
            None,
            PipelineKey::Mesh3D,
        )?;
        let sphere_id1 = context.spawn_sphere_3d(
            vec3(0.0, -1.0, 0.0),
            0.5,
            16,
            16,
            vec3(1.0, 0.0, 0.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;
        let sphere_id2 = context.spawn_sphere_3d(
            vec3(-10.0, -1.0, 0.0),
            0.5,
            16,
            16,
            vec3(1.0, 0.0, 0.0),
            0.5,
            None,
            PipelineKey::DebugLine3D,
        )?;
        let sphere_id3 = context.spawn_sphere_3d(
            vec3(-5.0, -1.0, 0.0),
            0.5,
            16,
            16,
            vec3(1.0, 0.0, 0.0),
            1.0,
            None,
            PipelineKey::Lit3D,
        )?;

        let line_id1 = context.spawn_line_3d(
            vec3(0.0, -20.0, 0.0),
            vec3(0.0, 20.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            1.0,
        )?;
        let line_id2 = context.spawn_line_3d(
            vec3(0.0, 0.0, -20.0),
            vec3(0.0, 0.0, 20.0),
            vec3(1.0, 1.0, 0.0),
            1.0,
        )?;
        let line_id3 = context.spawn_line_3d(
            vec3(-20.0, 0.0, 0.0),
            vec3(20.0, 0.0, 0.0),
            vec3(0.0, 1.0, 1.0),
            1.0,
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

    fn create_models(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let ghost_texture = context
            .texture("ghost")
            .unwrap_or(context.default_texture());

        let viking_room = context.spawn_model(
            "viking_room_lit3d",
            Transform {
                position: vec3(-5.0, 0.0, 2.0),
                ..Default::default()
            },
            Material {
                color: vec3(1.0, 0.0, 1.0),
                alpha: 1.0,
                use_texture: true,
                texture: ghost_texture,
                pipeline_key: PipelineKey::Lit3D,
            },
        );
        Ok(())
    }

    fn bind_input_commands(&mut self, context: &mut SceneContext<'_>) {
        context.bind_input_command(
            KeyCode::ArrowLeft,
            InputTrigger::Pressed,
            DespawnLastCommand,
        );
        context.bind_input_command(
            KeyCode::ArrowRight,
            InputTrigger::Pressed,
            SpawnVikingRoomCommand::default(),
        );
        context.bind_input_command(
            KeyCode::KeyT,
            InputTrigger::Pressed,
            SpawnPrimitiveCommand {
                primitive_type: PrimitiveType::Triangle,
                pipeline_key: PipelineKey::Lit3D,
                texture_name: Some("face"),
            },
        );
        context.bind_input_command(
            KeyCode::KeyR,
            InputTrigger::Pressed,
            SpawnPrimitiveCommand {
                primitive_type: PrimitiveType::Rectangle,
                pipeline_key: PipelineKey::Ui2D,
                texture_name: Some("face"),
            },
        );
        context.bind_input_command(
            KeyCode::KeyC,
            InputTrigger::Pressed,
            SpawnPrimitiveCommand {
                primitive_type: PrimitiveType::Cube,
                pipeline_key: PipelineKey::DebugLine3D,
                texture_name: None,
            },
        );
        context.bind_input_command(
            KeyCode::KeyI,
            InputTrigger::Pressed,
            SpawnPrimitiveCommand {
                primitive_type: PrimitiveType::Circle,
                pipeline_key: PipelineKey::Mesh3D,
                texture_name: None,
            },
        );
        context.bind_input_command(
            KeyCode::KeyP,
            InputTrigger::Pressed,
            SpawnPrimitiveCommand {
                primitive_type: PrimitiveType::Polygon,
                pipeline_key: PipelineKey::Mesh3D,
                texture_name: None,
            },
        );
        context.bind_input_command(
            KeyCode::KeyE,
            InputTrigger::Pressed,
            SpawnPrimitiveCommand {
                primitive_type: PrimitiveType::Sphere,
                pipeline_key: PipelineKey::Lit3D,
                texture_name: None,
            },
        );
        context.bind_input_command(
            KeyCode::KeyU,
            InputTrigger::Pressed,
            UpdatePrimitiveMeshesCommand,
        );
        context.bind_input_command(
            KeyCode::Digit1,
            InputTrigger::Pressed,
            CreateTriangleCommand {
                p0: vec3(0.0, 2.0, -0.3),
                p1: vec3(-7.0, 2.0, 0.3),
                p2: vec3(-2.0, 2.0, 1.0),
                color: vec3(1.0, 1.0, 0.0),
                alpha: 1.0,
                texture: Some("face"),
                pipeline_key: PipelineKey::Lit3D,
                scene_id: context.scene_id(),
            },
        );
        context.bind_input_command(
            KeyCode::Digit2,
            InputTrigger::Pressed,
            CreatePrimitiveCommand {
                primitive_shape: PrimitiveShape::Triangle {
                    points: [
                        vec3(0.0, 2.0, -0.3),
                        vec3(-7.0, 2.0, 0.3),
                        vec3(-2.0, 2.0, 1.0),
                    ],
                    color: vec3(1.0, 1.0, 1.0),
                },
                transform: Transform {
                    position: vec3(-5.0, 2.0, 0.0),
                    ..Default::default()
                },
                color: vec3(1.0, 1.0, 0.0),
                alpha: 1.0,
                texture: Some("face"),
                pipeline_key: PipelineKey::DebugLine3D,
                auto_release: true,
            },
        );
        context.bind_input_command(
            KeyCode::Space,
            InputTrigger::Pressed,
            ChangeSceneCommand {
                next_scene: "BasicFieldScene".to_string(),
            },
        );
        context.bind_input_command(KeyCode::Enter, InputTrigger::Pressed, DebugMonitor);
    }
}

struct CreateTriangleCommand {
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    color: Vec3,
    alpha: f32,
    texture: Option<&'static str>,
    pipeline_key: PipelineKey,
    scene_id: SceneId,
}

impl Command for CreateTriangleCommand {
    fn id(&self) -> String {
        format!("create_triangle")
    }
    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        let triangle = context.spawn_triangle_3d(
            self.p0,
            self.p1,
            self.p2,
            self.color,
            self.alpha,
            self.texture,
            self.pipeline_key,
        )?;
        context.add_component(
            triangle,
            SceneOwned {
                scene_id: self.scene_id,
            },
        );
        Ok(())
    }
}

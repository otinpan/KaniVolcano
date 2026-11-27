use anyhow::Result;
use cgmath::{vec3,vec2};
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;
use winit::keyboard::KeyCode;
use crate::{ThreadProfilerBootstrap, ChangeSceneCommand};
pub struct BasicFieldScene {
    scene_id: SceneId,
}

impl Scene for BasicFieldScene {
    fn name(&self) -> String {
        "BasicFieldScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        crate::system::FpsSystem::install(context)?;
        context.set_msaa(Msaa::X4);
        self.scene_id=context.scene_id();
        context.set_skybox("default")?;
        self.create_camera(context)?;
        self.create_fundation(context)?;
        self.create_models(context)?;
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
        context.add_component(camera, Transform::default())?;
        context.add_component(
            camera,
            Camera {
                up: vec3(0.0, 0.0, 1.0),
                // A valid initial direction is needed before CameraSystem's first update.
                target: vec3(-1.0, 0.0, 0.0),
                fov_y: 45.0,
                near: 0.1,
                far: 100.0,
                yaw: std::f32::consts::PI,
                pitch: 0.0,
            },
        )?;

        Ok(())
    }
    fn create_models(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let teacup=context.spawn_gltf_model(
            "teacup",
            Transform { 
                position: vec3(0.0,-5.0,0.0),
                rotation: vec3(45.0,0.0,0.0), 
                scale: vec3(2.0,2.0,2.0)
            },
            None,
        );

        let refrigerator = context.spawn_gltf_model(
            "refrigerator",
            Transform{
                position: vec3(0.0,-3.0,0.0),
                rotation: vec3(90.0,90.0,0.0),
                scale: vec3(1.0,1.0,1.0),
            },
            None,
        );
        
        let car=context.spawn_gltf_model(
            "car",
            Transform{
                position: vec3(0.0, 0.0,0.0),
                rotation: vec3(90.0,0.0,0.0),
                ..Default::default()
            },
            None,
        );

        let helmet=context.spawn_gltf_model(
            "helmet",
            Transform{
                position: vec3(0.0,5.0,0.0),
                rotation: vec3(90.0,0.0,0.0),
                ..Default::default()
            },
            None,
        );
        Ok(())
    }

    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("camera", CameraSystem);
    }

    fn create_fundation(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let sphere=context.spawn_sphere_3d(
            vec3(0.0,0.0,5.0),
            0.5,
            32,
            32,
            vec3(1.0,1.0,1.0),
            1.0,
            Some("texture_ref_test"),
            PipelineKey::Lit3D,
        )?;
        let cube=context.spawn_cube_3d(
            vec3(0.0,2.0,5.0),
            0.5,
            vec3(0.0,0.0,45.0),
            vec3(1.0,1.0,1.0),
            1.0,
            Some("texture_ref_test"),
            PipelineKey::Pbr3D,
        )?;
        let entities = [sphere, cube];
        context.bind_input_command(KeyCode::Backspace, InputTrigger::Pressed,
            TextureRefTestCommand { entities, clear_material: false });
        context.bind_input_command(KeyCode::KeyT, InputTrigger::Pressed,
            TextureRefTestCommand { entities, clear_material: true });
        println!("Texture test: refs=2. Backspace: despawn sphere/cube; T: remove one texture reference.");
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
        /*let hello_world=context.spawn_text_3d(
            "eng_font",
            "Hello\nKaniVolcano",
            48.0,
            60.0,
            vec3(-3.0,0.0,0.0),
            vec3(0.01,0.01,0.01),
            vec3(45.0,45.0,0.0),
            vec3(0.0,1.0,0.0),
            0.5,
        );*/

        Ok(())
    }

    fn bind_input_commands(&mut self, context: &mut SceneContext<'_>) {
        context.bind_input_command(
            KeyCode::Digit0,
            InputTrigger::Pressed,
            ChangeSceneCommand {
                next_scene: "AudioScene".to_string(),
            },
        );
        context.bind_input_command(
            KeyCode::Space,
            InputTrigger::Pressed,
            LoadAssetsCommand,
        );
        context.bind_input_command(
            KeyCode::Enter,
            InputTrigger::Pressed,
            SpawnTransparentVikingRoom,
        );
    }

    fn add_thread_profiler(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        // Both rectangles and grid lines reuse centered white 1x1 Ui2D meshes.
        // SceneContext creates their GPU meshes during the following render stage.
        let mut mesh_entities = Vec::new();
        for _ in 0..2 {
            let entity = context.spawn_rectangle_2d(
                vec2(0.0, 0.0),
                1.0,
                1.0,
                0.0,
                vec3(1.0, 1.0, 1.0),
                1.0,
                None,
            )?;
            if let Some(visibility) = context.get_component_mut::<Visibility>(entity) {
                visibility.is_visible = false;
            }
            mesh_entities.push(entity);
        }
        context.add_update_system(
            "thread_profiler",
            ThreadProfilerBootstrap {
                mesh_entities: [mesh_entities[0], mesh_entities[1]],
                scene_id: self.scene_id,
                profiler: None,
            },
        );
        Ok(())
    }
}

impl Default for BasicFieldScene {
    fn default() -> Self {
        Self {
            scene_id: SceneId(0),
        }
    }
}


pub struct LoadAssetsCommand;

// Restrict these operations to the two test primitives, preserving models and ground.
struct TextureRefTestCommand {
    entities: [EntityId; 2],
    clear_material: bool,
}

impl Command for TextureRefTestCommand {
    fn id(&self) -> String {
        format!("texture_ref_test_{}", self.clear_material)
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        for &entity in &self.entities {
            if self.clear_material {
                let Some(mut material) = context.get_component::<MeshRenderer>(entity)
                    .map(|renderer| renderer.material)
                    .filter(|material| material.base_color_texture.is_some()) else { continue };
                material.base_color_texture = None;
                context.set_material(entity, material)?;
                println!("Texture test: removed texture from {entity:?}");
            } else {
                if !context.despawn(entity) { continue; }
                println!("Texture test: despawned {entity:?}");
            }
            break;
        }
        match context.texture_asset_id("texture_ref_test") {
            Ok(id) => {
                if let Some((_, asset)) = context.texture_assets().find(|(key, _)| *key == id) {
                    println!("Texture test: refs={}, auto_release={}", asset.ref_count, asset.auto_release);
                }
            }
            Err(_) => println!("Texture test: asset removed; GPU destruction queued (or already processed)."),
        }
        Ok(())
    }
}

impl Command for LoadAssetsCommand{
    fn id(&self) -> String{
        format!("load_assets")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>{
        context.request_load_texture("jupiter","assets/textures/jupiter.png", false);
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
        let jupiter=context.texture_asset_id("jupiter")?;
        context.spawn_model(
            "viking_room_lit3d",
            Transform { 
                position: vec3(0.0,0.0,3.0), 
                rotation: vec3(0.0,45.0,0.0),
                scale: vec3(1.0,1.0,1.0)
            },
            Material { 
                base_color: vec3(1.0,1.0,1.0).extend(0.5),
                base_color_texture: Some(jupiter),
                pipeline_key: PipelineKey::Lit3D,
                ..Material::default()
            }
        )?;
        Ok(())
    }
}

// Rotation work per job is too small to offset job scheduling, synchronization,
// and fiber switching overhead. Collecting query results also runs serially,
// making this implementation slower than RotatorSystem in our measurements.
use super::ChangeSceneCommand;
use crate::system::ParallelRotatorSystem;
use anyhow::{Result, anyhow};
use cgmath::vec3;
use kani_volcano_engine::prelude::*;
use rand::{RngExt, rngs::ThreadRng};
use winit::keyboard::KeyCode;

pub struct ParallelRotatorScene {
    scene_id: SceneId,
    main_cube: Option<EntityId>,
    ground: f32,
    rng: ThreadRng,
    cubes_created: bool,
}

impl Scene for ParallelRotatorScene {
    fn name(&self) -> String {
        "ParallelRotatorScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        self.scene_id = context.scene_id();
        self.main_cube = None;
        self.cubes_created = false;
        context.set_skybox("default")?;
        self.create_camera(context)?;
        self.create_fundation(context)?;
        self.create_texts(context)?;
        self.create_rotate_entities(context)?;
        //self.spawn_rotate_primitive(context)?;

        self.add_update_systems(context);
        self.bind_input_commands(context);
        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        /*if self.cubes_created {
            return Ok(());
        }
        let Some(cube) = self.main_cube else {
            return Ok(());
        };
        let Some(mesh_id) = context.mesh_asset_id(cube) else {
            return Ok(());
        };
        self.create_rotate_cubes(context, mesh_id)?;
        self.cubes_created = true;*/
        Ok(())
    }

    fn on_exit(&mut self, _context: &mut SceneContext<'_>) -> Result<()> {
        Ok(())
    }
}

impl ParallelRotatorScene {
    fn create_camera(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let camera = context.spawn();
        context.add_component(camera, Transform::default());
        let success = context.add_component(
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
        );

        if !success {
            Err(anyhow!("failed to create Camera"))
        } else {
            Ok(())
        }
    }

    fn create_fundation(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.spawn_rectangle_3d(
            vec3(0.0, 0.0, self.ground),
            40.0,
            40.0,
            vec3(0.0, -90.0, 0.0),
            vec3(0.5, 0.5, 0.5),
            1.0,
            None,
            PipelineKey::Mesh3D,
        )?;

        Ok(())
    }

    fn spawn_rotate_primitive(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let size = 0.5;
        let cube = context.spawn_cube_3d(
            vec3(0.0, 0.0, 0.0),
            size,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 1.0),
            1.0,
            None,
            PipelineKey::Mesh3D,
        )?;

        self.main_cube = Some(cube);
        if let Some(visibility) = context.get_component_mut::<Visibility>(cube) {
            visibility.is_visible = false;
        }
        Ok(())
    }

    fn create_rotate_entities(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        for _ in 0..100_000 {
            let entity = context.spawn();

            context.add_component(entity, Transform::default());
            context.add_component(entity, Rotator {
                speed: vec3(60.0, 0.0, 0.0),
            });
            context.add_component(entity, SceneOwned {
                scene_id: self.scene_id,
            });
        }

        Ok(())
    }
    fn create_rotate_cubes(
        &mut self,
        context: &mut UpdateContext<'_>,
        mesh_id: MeshAssetId,
    ) -> Result<()> {
        let texture = context.default_texture();
        for _ in 0..200{
            let height = self.rng.random_range(1.0..5.0);
            let width = self.rng.random_range(0.5..2.0);
            let z=self.rng.random_range(0.0..5.0);
            let y = self.rng.random_range(-17.0..17.0);
            let x = self.rng.random_range(-17.0..17.0);
            let rot=self.rng.random_range(-60.0..60.0);

            let obj = context.spawn_primitive_from_mesh(
                mesh_id,
                Material {
                    color: vec3(0.7, 0.7, 0.7),
                    alpha: 1.0,
                    use_texture: false,
                    texture: texture,
                    pipeline_key: PipelineKey::Mesh3D,
                },
                Transform {
                    position: vec3(x, y, self.ground + z),
                    rotation: vec3(0.0, 0.0, 0.0),
                    scale: vec3(width, width, height),
                },
            )?;
            context.add_component(
                obj,
                Rotator {
                    speed: vec3(rot, 0.0, 0.0),
                },
            );
            context.set_tags(obj, ["rotator"]);
            context.add_component(
                obj,
                SceneOwned {
                    scene_id: self.scene_id,
                },
            );
        }
        Ok(())
    }

    fn create_texts(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.spawn_text_3d(
            "eng_font",
            "ParallelRotatorScene",
            100.0,
            100.0,
            vec3(-3.0, -3.0, 5.0),
            vec3(0.01, 0.01, 0.01),
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 1.0),
            1.0,
        )?;
        Ok(())
    }

    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("camera",CameraSystem);
        context.add_fixed_update_system("parallel_rotator_system", ParallelRotatorSystem::default());
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

impl Default for ParallelRotatorScene {
    fn default() -> Self {
        Self {
            scene_id: SceneId(0),
            main_cube: None,
            ground: -1.0,
            rng: rand::rng(),
            cubes_created: false,
        }
    }
}

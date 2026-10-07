use super::ChangeSceneCommand;
use crate::system::{AudioSystem, MoveRotateSystem};
use crate::component::{MoveRotateComponent};
use anyhow::{Result, anyhow};
use cgmath::{vec2, vec3};
use kani_volcano_audio::ReverbSettings;
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;
use std::collections::HashMap;
use winit::keyboard::KeyCode;

pub struct AudioScene {
    scene_id: SceneId,
    main_cube: Option<EntityId>,
    ground: f32,
    // emitter, output_bus, buses
    buses: HashMap<String, (String, Vec<String>)>,
}

impl Scene for AudioScene {
    fn name(&self) -> String {
        "AudioScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        self.scene_id = context.scene_id();
        context.set_skybox("default")?;
        self.create_camera(context)?;
        self.create_fundation(context)?;
        self.create_texts(context)?;
        self.create_cuboid(context)?;
        //self.load_audio(context)?;
        self.create_buses(context)?;

        // Names are available before the deferred audio registration completes.
        self.buses = (0..2)
            .map(|index| {
                (
                    format!("emitter{index}"),
                    (
                        format!("direct_{index}"),
                        ["water", "wood", "steel"]
                            .into_iter()
                            .map(|tag| format!("{tag}_{index}"))
                            .collect(),
                    ),
                )
            })
            .collect();
        self.add_update_systems(context);

        self.bind_input_commands(context);
        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let Some(cube) = self.main_cube else {
            return Ok(());
        };
        let Some(mesh_id) = context.mesh_asset_id(cube) else {
            return Ok(());
        };
        if !self.create_emitters(context)? {
            return Ok(());
        }
        if self.create_audio_objects(context, mesh_id)? {
            self.create_blocking_objects(context, mesh_id)?;
            self.play_audio(context)?;
            self.main_cube = None;
        }
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        self.main_cube = None;
        // Emitters must be removed before the buses they reference.
        for index in 0..2 {
            let name = format!("emitter{index}");
            if context.audio_emitter_id(&name).is_ok() {
                context.destroy_emitter(&name)?;
            }
        }
        for index in 0..2 {
            for kind in ["direct", "water", "wood", "steel"] {
                let name = format!("{kind}_{index}");
                if context.audio_bus_id(&name).is_ok() {
                    context.destroy_bus(&name)?;
                }
            }
        }
        Ok(())
    }
}

impl AudioScene {
    fn create_camera(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let camera = context.spawn();
        context.add_component(camera, Transform::default());
        let success = context.add_component(
            camera,
            Camera {
                target: vec3(-5.0, 0.0, 0.0),
                up: vec3(0.0, 0.0, 1.0),
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

    fn load_audio(&self, context: &mut SceneContext<'_>) -> Result<()> {
        context.request_load_audio("neko", "assets/sounds/sample_sound.mp3");
        context.request_load_audio("flog", "assets/sounds/sample_sound2.mp3");
        Ok(())
    }

    fn create_buses(&self, context: &mut SceneContext<'_>) -> Result<()> {
        let master = context.master_bus()?;
        for index in 0..2 {
            context.create_bus(
                &format!("direct_{index}"),
                AudioBusSettings {
                    volume: 1.0,
                    panning: 0.0,
                    bus_type: AudioBusType::Sub { output: master },
                },
            );
            // Starting values for material effects; tune these by ear.
            for (material, feedback, damping, stereo_width, volume) in [
                ("water", 0.7, 0.8, 0.5, 0.0),
                ("wood", 0.35, 0.7, 0.3, 0.0),
                ("steel", 0.8, 0.2, 0.8, 0.0),
            ] {
                context.create_bus(
                    &format!("{material}_{index}"),
                    AudioBusSettings {
                        // A blocking system can fade in each effect independently.
                        volume,
                        panning: 0.0,
                        bus_type: AudioBusType::Reverb {
                            settings: ReverbSettings {
                                feedback,
                                damping,
                                stereo_width,
                            },
                        },
                    },
                );
            }
        }
        Ok(())
    }

    fn create_emitters(&mut self, context: &mut UpdateContext<'_>) -> Result<bool> {
        let mut ready = true;
        for index in 0..2 {
            let name = format!("emitter{index}");
            // Audio registration runs between updates; skip registered emitters.
            if context.audio_emitter_id(&name).is_ok() {
                continue;
            }
            ready = false;
            let (Ok(output), Ok(water), Ok(wood), Ok(steel)) = (
                context.audio_bus_id(&format!("direct_{index}")),
                context.audio_bus_id(&format!("water_{index}")),
                context.audio_bus_id(&format!("wood_{index}")),
                context.audio_bus_id(&format!("steel_{index}")),
            ) else {
                continue;
            };
            context.create_emitter(
                &name,
                AudioEmitterSettings {
                    output,
                    volume: 1.0,
                    panning: 0.0,
                    sends: vec![
                        AudioSend {
                            bus: water,
                            gain: 1.0,
                        },
                        AudioSend {
                            bus: wood,
                            gain: 1.0,
                        },
                        AudioSend {
                            bus: steel,
                            gain: 1.0,
                        },
                    ],
                },
            );
        }
        Ok(ready)
    }

    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("camera", CameraSystem);
        context.add_update_system(
            "audio",
            AudioSystem {
                buses: self.buses.clone(),
            },
        );
        context.add_fixed_update_system("rotator", MoveRotateSystem);
    }

    fn create_fundation(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.spawn_rectangle_3d(
            vec3(0.0, 0.0, self.ground),
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

    fn create_cuboid(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let height = 1.0;
        let cuboid = context.spawn_cuboid_3d(
            vec3(0.0, 0.0, height / 2.0),
            1.0,
            1.0,
            height,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 1.0),
            1.0,
            None,
            PipelineKey::Lit3D,
        )?;
        self.main_cube = Some(cuboid);
        if let Some(visibility) = context.get_component_mut::<Visibility>(cuboid) {
            visibility.is_visible = false;
        }
        Ok(())
    }

    fn create_audio_objects(
        &self,
        context: &mut UpdateContext<'_>,
        mesh_id: MeshAssetId,
    ) -> Result<bool> {
        // Resolve every dependency before spawning either bill.
        let (Ok(neko), Ok(flog), Ok(emitter0), Ok(emitter1)) = (
            context.audio_asset_id("neko"),
            context.audio_asset_id("flog"),
            context.audio_emitter_id("emitter0"),
            context.audio_emitter_id("emitter1"),
        ) else {
            return Ok(false);
        };
        let height = 7.0;
        let width = 2.0;
        let texture = context.default_texture();
        let items = [
            // position, color, audio, emitter
            (vec2(-8.0, -8.0), vec3(1.0, 1.0, 0.0), neko, emitter0),
            (vec2(8.0, 8.0), vec3(0.0, 1.0, 1.0), flog, emitter1),
        ];
        for item in items {
            let bill = context.spawn_primitive_from_mesh(
                mesh_id,
                Material {
                    color: item.1,
                    alpha: 1.0,
                    use_texture: false,
                    texture: texture,
                    pipeline_key: PipelineKey::Lit3D,
                },
                Transform {
                    position: vec3(item.0.x, item.0.y, self.ground + height / 2.0 ),
                    rotation: vec3(0.0, 0.0, 0.0),
                    scale: vec3(width, width, height),
                },
            )?;

            context.add_component(
                bill,
                MoveRotateComponent{
                    center: vec3(0.0,0.0,0.0),
                    up: vec3(0.0,0.0,1.0),
                    clock_wise: true,
                    speed: std::f32::consts::FRAC_PI_2/4.0, // 4.0secs -> 90rad
                }
            );

            context.add_component(
                bill,
                SceneOwned {
                    scene_id: self.scene_id,
                },
            );
            context.add_component(
                bill,
                AudioSource {
                    audio: item.2,
                    emitter: Some(item.3),
                    settings: Default::default(),
                },
            );
        }
        Ok(true)
    }

    fn create_blocking_objects(
        &self,
        context: &mut UpdateContext<'_>,
        mesh_id: MeshAssetId,
    ) -> Result<()> {
        let texture = context.default_texture();
        let items = [
            // position, size, color, alpha, Pipeline, name
            (
                vec2(-5.0,5.0),
                3.0,
                vec3(0.4, 0.4, 1.0),
                0.5,
                PipelineKey::Lit3D,
                "water",
            ),
            (
                vec2(0.0, 0.0),
                3.0,
                vec3(0.4, 0.2, 0.0),
                1.0,
                PipelineKey::Lit3D,
                "wood",
            ),
            (
                vec2(5.0, -5.0),
                3.0,
                vec3(0.1, 0.1, 0.1),
                1.0,
                PipelineKey::Lit3D,
                "steel",
            ),
        ];

        for item in items {
            let size = item.1;
            let obj = context.spawn_primitive_from_mesh(
                mesh_id,
                Material {
                    color: item.2,
                    alpha: item.3,
                    use_texture: false,
                    texture: texture,
                    pipeline_key: item.4,
                },
                Transform {
                    position: vec3(item.0.x, item.0.y, self.ground + size / 2.0+0.3),
                    scale: vec3(size, size, size),
                    ..Default::default()
                },
            )?;

            context.set_tags(obj, [item.5]);
            // Approximate the cube with an inscribed sphere in world units.
            context.add_component(
                obj,
                crate::component::SphereColliderComponent { radius: size / 2.0 },
            );
            context.add_component(
                obj,
                SceneOwned {
                    scene_id: self.scene_id,
                },
            );
        }

        Ok(())
    }

    fn play_audio(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let playback0 = context.play_on_emitter(
            "neko",
            "emitter0",
            PlaybackSettings {
                volume: 1.0,
                looping: true,
            },
        );
        let playback1 = context.play_on_emitter(
            "flog",
            "emitter1",
            PlaybackSettings {
                volume: 1.0,
                looping: true,
            },
        );

        Ok(())
    }

    fn create_texts(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        let audio_text=context.spawn_text_3d(
            "eng_font",
            "AudioScene",
            100.0,
            100.0,
            vec3(-3.0,-3.0,4.0),
            vec3(0.01,0.01,0.01),
            vec3(0.0,0.0,0.0),
            vec3(1.0,1.0,1.0),
            1.0,
        );
        Ok(())
    }

    fn bind_input_commands(&mut self, context: &mut SceneContext<'_>) {
        context.bind_input_command(
            KeyCode::Space,
            InputTrigger::Pressed,
            ChangeSceneCommand {
                next_scene: "ParallelRotatorScene".to_string(),
            },
        )
    }
}

impl Default for AudioScene {
    fn default() -> Self {
        Self {
            scene_id: SceneId(0),
            main_cube: None,
            ground: -1.0,
            buses: HashMap::new(),
        }
    }
}

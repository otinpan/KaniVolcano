use anyhow::{Result, anyhow};
use cgmath::{vec2, vec3};
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;
use winit::keyboard::KeyCode;

use crate::ChangeSceneCommand;

#[derive(Default)]
pub struct MaterialScene;

impl Scene for MaterialScene {
    fn name(&self) -> String {
        "MaterialScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        self.create_camera(context)?;
        self.create_spheres(context)?;
        context.add_update_system("camera", CameraSystem);
        context.bind_input_command(
            KeyCode::Digit0,
            InputTrigger::Pressed,
            ChangeSceneCommand { next_scene: "Basic3dScene".to_string() },
        );
        context.spawn_text_ui2d(
            "eng_font",
            "PBR MATERIALS | top: roughness / middle: metallic / bottom: texture + emission\nWASD: move | RMB: look | arrows up/down: height | 0: back\nDirect lighting only: no environment reflections or bloom.",
            18.0, 24.0, vec2(-0.95, -0.95), vec2(1.0, 1.0), 0.0,
            vec3(0.9, 0.95, 1.0), 1.0,
        )?;
        Ok(())
    }

    fn update(&mut self, _context: &mut UpdateContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_exit(&mut self, _context: &mut SceneContext<'_>) -> Result<()> {
        Ok(())
    }
}

impl MaterialScene {
    fn create_camera(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let entity = context.spawn();
        context.add_component(entity, Transform {
            position: vec3(4.0, 0.0, 1.0),
            ..Transform::default()
        });
        if !context.add_component(entity, Camera {
            up: vec3(0.0, 0.0, 1.0),
            target: vec3(-5.0, 0.0, 1.0),
            fov_y: 55.0,
            near: 0.1,
            far: 100.0,
            yaw: std::f32::consts::PI,
            pitch: 0.0,
        }) {
            return Err(anyhow!("failed to create material comparison camera"));
        }
        Ok(())
    }

    fn create_spheres(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let base = Material {
            pipeline_key: PipelineKey::Pbr3D,
            base_color: vec3(0.8, 0.55, 0.22).extend(1.0),
            metallic: 0.0,
            roughness: 0.35,
            ..Material::default()
        };

        // Keep color and metalness fixed while varying roughness.
        for (column, roughness) in [0.08, 0.25, 0.55, 1.0].into_iter().enumerate() {
            Self::spawn_sample(context, column, 3.2, Material {
                metallic: 1.0,
                roughness,
                ..base
            }, &format!("R={roughness:.2}  M=1"))?;
        }

        // Keep color and roughness fixed while varying metalness.
        for (column, metallic) in [0.0, 0.33, 0.67, 1.0].into_iter().enumerate() {
            Self::spawn_sample(context, column, 1.0, Material {
                metallic,
                ..base
            }, &format!("M={metallic:.2}  R=0.35"))?;
        }

        let white = Material {
            base_color: vec3(1.0, 1.0, 1.0).extend(1.0),
            roughness: 0.55,
            ..base
        };
        Self::spawn_sample(context, 0, -1.2, white, "No texture")?;
        Self::spawn_sample(context, 1, -1.2, Material {
            base_color_texture: Some(context.texture("face")?),
            ..white
        }, "face texture")?;

        let emissive_base = Material {
            base_color: vec3(0.03, 0.18, 0.24).extend(1.0),
            emissive_color: vec3(0.02, 0.45, 0.65),
            emissive_strength: 0.0,
            roughness: 0.55,
            ..base
        };
        Self::spawn_sample(context, 2, -1.2, emissive_base, "Emission=0")?;
        Self::spawn_sample(context, 3, -1.2, Material {
            emissive_strength: 1.0,
            ..emissive_base
        }, "Emission=1")?;
        Ok(())
    }

    fn spawn_sample(
        context: &mut SceneContext<'_>,
        column: usize,
        height: f32,
        material: Material,
        label: &str,
    ) -> Result<()> {
        let y = (column as f32 - 1.5) * 2.1;
        // White vertex colors avoid tinting the material a second time.
        let entity = context.spawn_sphere_3d(
            vec3(-5.0, y, height), 0.65, 32, 48,
            vec3(1.0, 1.0, 1.0), 1.0, None, PipelineKey::Pbr3D,
        )?;
        context.set_material(entity, material)?;
        context.set_name(entity, label);
        context.spawn_text_3d(
            "eng_font", label, 24.0, 28.0,
            vec3(-4.25, y - 0.8, height - 0.85),
            vec3(0.006, 0.006, 0.006), vec3(0.0, 0.0, 0.0),
            vec3(0.9, 0.95, 1.0), 1.0,
        )?;
        Ok(())
    }
}

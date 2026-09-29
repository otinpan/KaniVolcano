use anyhow::Result;
use cgmath::{InnerSpace, vec3};
use kani_volcano_math::Transform;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use super::{UpdateContext, UpdateSystem};
use crate::{Camera, EntityApi, InputApi, TimeApi};

#[derive(Clone, Debug)]
pub struct CameraSystem;

impl CameraSystem {
    fn update_camera(&mut self, context: &mut UpdateContext<'_>, delta_time: f32) -> Result<()> {
        let move_speed = 3.0;
        let mouse_sensitivity = 0.003;
        let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
        let mouse_delta = context.mouse_delta();
        let right_mouse_down = context.mouse_button_down(MouseButton::Right);
        let move_forward = context.key_down(KeyCode::KeyW);
        let move_backward = context.key_down(KeyCode::KeyS);
        let move_left = context.key_down(KeyCode::KeyA);
        let move_right = context.key_down(KeyCode::KeyD);
        let move_up = context.key_down(KeyCode::ArrowUp);
        let move_down = context.key_down(KeyCode::ArrowDown);

        if let Some((_, transform, camera)) = context.query2_mut_mut::<Transform, Camera>().next() {
            if right_mouse_down {
                camera.yaw -= mouse_delta.x * mouse_sensitivity;
                camera.pitch -= mouse_delta.y * mouse_sensitivity;
                camera.pitch = camera.pitch.clamp(-max_pitch, max_pitch);
            }

            anyhow::ensure!(camera.yaw.is_finite() && camera.pitch.is_finite(),
                "camera yaw and pitch must be finite");
            camera.pitch = camera.pitch.clamp(-max_pitch, max_pitch);
            let up = camera.normalized_up()?;
            let direction = camera_direction(up, camera.yaw, camera.pitch);
            let left = up.cross(direction).normalize();

            if move_forward {
                transform.position += direction * move_speed * delta_time;
            }
            if move_backward {
                transform.position -= direction * move_speed * delta_time;
            }
            if move_left {
                transform.position += left * move_speed * delta_time;
            }
            if move_right {
                transform.position -= left * move_speed * delta_time;
            }
            if move_up {
                transform.position += up * move_speed * delta_time;
            }
            if move_down {
                transform.position -= up * move_speed * delta_time;
            }

            camera.target = transform.position + direction;
        }

        Ok(())
    }
}

// Project +X onto the horizontal plane to preserve the existing Z-up yaw convention.
// Near an X-aligned up direction, use +Y as the reference instead.
fn camera_direction(up: cgmath::Vector3<f32>, yaw: f32, pitch: f32) -> cgmath::Vector3<f32> {
    let reference = if up.x.abs() < 0.99 { vec3(1.0, 0.0, 0.0) } else { vec3(0.0, 1.0, 0.0) };
    let forward = (reference - up * reference.dot(up)).normalize();
    let side = up.cross(forward);
    ((forward * yaw.cos() + side * yaw.sin()) * pitch.cos() + up * pitch.sin()).normalize()
}

impl UpdateSystem for CameraSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let delta_time = context.delta_seconds();

        self.update_camera(context, delta_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_up_preserves_existing_direction() {
        let (yaw, pitch) = (0.7_f32, -0.3_f32);
        let expected = vec3(yaw.cos() * pitch.cos(), yaw.sin() * pitch.cos(), pitch.sin());
        assert!((camera_direction(vec3(0.0, 0.0, 1.0), yaw, pitch) - expected).magnitude() < 1e-6);
    }

    #[test]
    fn arbitrary_up_defines_pitch_and_horizontal_movement() {
        for up in [vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(1.0, 2.0, 3.0).normalize()] {
            let direction = camera_direction(up, 0.7, 0.3);
            assert!((direction.magnitude() - 1.0).abs() < 1e-6);
            assert!((direction.dot(up) - 0.3_f32.sin()).abs() < 1e-6);
            let left = up.cross(direction).normalize();
            assert!(left.dot(up).abs() < 1e-6);
            assert!(left.dot(direction).abs() < 1e-6);
        }
    }

    #[test]
    fn invalid_view_basis_is_rejected() {
        let mut camera = Camera {
            target: vec3(1.0, 0.0, 0.0), up: vec3(0.0, 0.0, 2.0),
            fov_y: 60.0, near: 0.1, far: 100.0, yaw: 0.0, pitch: 0.0,
        };
        let position = vec3(0.0, 0.0, 0.0);
        assert_eq!(camera.view_up(position).unwrap(), vec3(0.0, 0.0, 1.0));
        for up in [position, vec3(f32::NAN, 0.0, 1.0), vec3(1.0, 0.0, 0.0)] {
            camera.up = up;
            assert!(camera.view_up(position).is_err());
        }
        camera.up = vec3(0.0, 0.0, 1.0);
        camera.target = position;
        assert!(camera.view_up(position).is_err());
    }
}

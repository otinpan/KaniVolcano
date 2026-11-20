use cgmath::Matrix4;
use super::Component;

/// World matrix computed by TransformSystem. Edit Transform for local changes.
#[derive(Clone, Copy, Debug)]
pub struct GlobalTransform {
    pub matrix: Matrix4<f32>,
}

impl Component for GlobalTransform {}

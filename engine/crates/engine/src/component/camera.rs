use super::Component;
// Camera ///////////////////////////////////////
#[derive(Clone, Debug)]
pub struct Camera {
    pub target: cgmath::Vector3<f32>,
    /// Reference up direction for the view and camera controls. Must be finite and nonzero.
    pub up: cgmath::Vector3<f32>,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Component for Camera {}

impl Camera {
    pub(crate) fn normalized_up(&self) -> anyhow::Result<cgmath::Vector3<f32>> {
        use cgmath::InnerSpace;
        let length_squared = self.up.magnitude2();
        anyhow::ensure!(length_squared.is_finite() && length_squared > 1e-12,
            "camera up must be finite and nonzero");
        Ok(self.up.normalize())
    }

    pub(crate) fn view_up(&self, position: cgmath::Vector3<f32>) -> anyhow::Result<cgmath::Vector3<f32>> {
        use cgmath::InnerSpace;
        let up = self.normalized_up()?;
        let forward = self.target - position;
        let length_squared = forward.magnitude2();
        anyhow::ensure!(length_squared.is_finite() && length_squared > 1e-12,
            "camera target must differ from its position and define a finite direction");
        anyhow::ensure!(up.cross(forward / length_squared.sqrt()).magnitude2() > 1e-8,
            "camera up must not be parallel to the view direction");
        Ok(up)
    }
}

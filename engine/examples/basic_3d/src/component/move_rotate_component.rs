use kani_volcano_engine::*;
use cgmath::{Vector3,};
pub struct MoveRotateComponent{
    pub center: Vector3<f32>, 
    pub up: Vector3<f32>, 
    pub clock_wise: bool,
    pub speed: f32,
}

impl Component for MoveRotateComponent{}
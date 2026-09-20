use crate::{Component, FontAssetId};
use cgmath::Vector3;

pub struct Text{
    pub content: String,
    pub font: FontAssetId,
    pub font_size: f32,
    pub line_height: f32,
    pub color: Vector3<f32>,
    pub alpha: f32,
}

impl Component for Text{}
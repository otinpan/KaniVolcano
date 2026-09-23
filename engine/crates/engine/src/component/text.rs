use crate::{Component, FontAssetId};
use cgmath::Vector3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextPipeline{
    Ui2D,
    World3D,
}
pub struct Text{
    pub content: String,
    pub font: FontAssetId,
    pub font_size: f32,
    pub line_height: f32,
    pub color: Vector3<f32>,
    pub alpha: f32,
    pub text_pipeline: TextPipeline,
}

impl Component for Text{}
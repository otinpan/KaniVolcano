use gltf::{
    Accessor, Animation, Camera, Material, Mesh, Node, Scene, Skin, Texture,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelAssetId(pub usize);

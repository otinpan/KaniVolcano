mod loader;
mod types;
pub use gltf::image::Format as ModelImageFormat;
pub use gltf::texture::{MagFilter, MinFilter, WrappingMode};
pub use loader::load_gltf;
pub use types::AlphaMode as ModelAlphaMode;

pub use types::{
    GltfModel, ModelScene, ModelNode, ModelMesh, ModelMaterial, ModelTexture,
    ModelImage, ModelSkin, ModelAnimation, NodeTransform, ModelPrimitive, PrimitiveTopology,
};


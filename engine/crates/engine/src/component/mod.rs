#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod camera;
mod material;
mod mesh_renderer;
mod name;
mod pending_primitive_mesh;
mod rotator;
mod scene_owned;
mod tags;
mod text;
mod visibility;

pub use camera::Camera;
pub use camera::Camera as CameraComponent;
use kani_volcano_math::Transform;
pub use material::Material;
pub use mesh_renderer::MeshRenderer;
pub use name::Name;
pub use pending_primitive_mesh::PendingPrimitiveMesh;
pub use rotator::Rotator;
pub use scene_owned::{SceneId, SceneOwned};
pub use tags::Tags;
pub use text::Text;
pub use visibility::Visibility;

// Component does not have short reference like &'a str, &String
pub trait Component: 'static {}

impl Component for Transform {}

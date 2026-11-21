#[derive(Debug, Default)]
pub struct GltfModel{
    pub scenes: Vec<ModelScene>,
    pub default_scene: Option<usize>,
    pub nodes: Vec<ModelNode>,
    pub meshes: Vec<ModelMesh>,
    pub materials: Vec<ModelMaterial>,
    pub textures: Vec<ModelTexture>,
    pub images: Vec<ModelImage>,
    pub skins: Vec<ModelSkin>,
    pub animations: Vec<ModelAnimation>,
}


#[derive(Debug, Default)]
pub struct ModelScene{
    pub name: Option<String>,
    pub roots: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct ModelNode{
    pub name: Option<String>,
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
    pub skin: Option<usize>,
    pub transform: NodeTransform,
}

#[derive(Debug, Default)]
pub struct ModelMesh{
    pub name: Option<String>,
    pub primitives: Vec<ModelPrimitive>,
}

#[derive(Debug, Default)]
pub struct ModelPrimitive{
    pub positions: Vec<[f32; 3]>,
    pub normal: Option<Vec<[f32; 3]>>,
    pub tangents: Option<Vec<[f32;4]>>,
    /// JOINTS_0: indices into ModelSkin.joints, not node indices.
    pub joints: Option<Vec<[u16; 4]>>,
    /// WEIGHTS_0: four influences per vertex; present together with joints.
    pub weights: Option<Vec<[f32; 4]>>,
    /// Currently stores TEXCOORD_0 only.
    pub tex_coord: Option<Vec<[f32; 2]>>,
    pub indices: Option<Vec<u32>>,
    pub material: Option<usize>,
    pub topology: PrimitiveTopology,
}


#[derive(Debug)]
pub enum NodeTransform{
    Trs{
        translation: [f32; 3],
        rotation: [f32; 4],
        scale: [f32; 3],
    },
    Matrix([[f32; 4]; 4])
}

#[derive(Debug, Default)]
pub enum PrimitiveTopology{
    Points,
    Lines,
    #[default]
    Triangles,
}

#[derive(Debug)]
pub struct ModelMaterial{
    pub name: Option<String>,

    pub base_color: [f32; 4],
    pub base_color_texture: Option<TextureRef>,

    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: Option<TextureRef>,

    pub normal_texture: Option<TextureRef>,
    pub normal_scale: f32,

    pub occlusion_texture: Option<TextureRef>,
    pub occlusion_strength: f32,

    pub emissive: [f32; 3],
    pub emissive_texture: Option<TextureRef>,
    pub emissive_strength: f32,

    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub transmission_factor: f32,
    pub double_sided: bool,
}


#[derive(Debug, Default)]
pub struct TextureRef{
    pub texture: usize, // GltfModel.textures
    /// Only set 0 is currently supported by ModelPrimitive.
    pub tex_coord: u32,
}

#[derive(Debug, Default)]
pub enum AlphaMode{
    #[default]
    Opaque,
    Mask,
    Blend,
}

#[derive(Debug, Default)]
pub struct ModelTexture{
    pub image: usize, // GltfModel.images
    pub sampler: ModelSampler,
}

#[derive(Debug)]
pub struct ModelSampler{
    pub mag_filter: Option<gltf::texture::MagFilter>,
    pub min_filter: Option<gltf::texture::MinFilter>,
    pub wrap_s: gltf::texture::WrappingMode,
    pub wrap_t: gltf::texture::WrappingMode,
}

#[derive(Debug, Default)]
pub struct ModelAnimation{
    pub name: Option<String>,
    pub duration: f32,
    pub tracks: Vec<AnimationTrack>,
}

#[derive(Debug, Default)]
pub struct AnimationTrack{
    pub node: usize,
    pub time: Vec<f32>,
    pub interpolation: Interpolation,
    pub values: AnimationValues,
}


#[derive(Debug, Default)]
pub enum Interpolation{
    Step,
    #[default]
    Linear,
    CubicSpline,
}

/// Node TRS animation only. The loader must explicitly reject unsupported
/// morph targets/weight animation rather than silently dropping them.
/// CubicSpline stores [in_tangent, value, out_tangent] for each key time.
#[derive(Debug)]
pub enum AnimationValues{
    Translation(Vec<[f32;3]>),
    Rotation(Vec<[f32;4]>),
    Scale(Vec<[f32; 3]>),
}

#[derive(Debug, Default)]
pub struct ModelSkin{
    pub name: Option<String>,
    pub joints: Vec<usize>, // nodes
    pub inverse_bind_matrices: Vec<[[f32; 4]; 4]>,
    pub skeleton: Option<usize>,
}

pub type ModelImage=gltf::image::Data;

impl Default for NodeTransform {
    fn default() -> Self {
        Self::Trs {
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
        }
    }
}

impl Default for ModelMaterial {
    fn default() -> Self {
        Self {
            name: None,
            base_color: [1.0; 4],
            base_color_texture: None,
            metallic: 1.0,
            roughness: 1.0,
            metallic_roughness_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            occlusion_texture: None,
            occlusion_strength: 1.0,
            emissive: [0.0; 3],
            emissive_texture: None,
            emissive_strength: 1.0,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            transmission_factor: 0.0,
            double_sided: false,
        }
    }
}

impl Default for ModelSampler {
    fn default() -> Self {
        Self {
            mag_filter: None,
            min_filter: None,
            wrap_s: gltf::texture::WrappingMode::Repeat,
            wrap_t: gltf::texture::WrappingMode::Repeat,
        }
    }
}

impl Default for AnimationValues {
    fn default() -> Self {
        Self::Translation(Vec::new())
    }
}

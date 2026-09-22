use cgmath::{Matrix4, vec3};
use vulkanalia::prelude::v1_0::*;

use super::model::MeshData;
use super::vertex::{DebugLineVertex, Mesh3DVertex, SourceVertex, VertexLayout};
use kani_volcano_math::Transform;

#[derive(Clone, Debug, Default)]
pub struct VulkanData {
    // Debug
    pub messenger: vk::DebugUtilsMessengerEXT,
    // Surface
    pub surface: vk::SurfaceKHR,
    // Physical Device
    pub physical_device: vk::PhysicalDevice,
    pub msaa_samples: vk::SampleCountFlags,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    // Swapchain
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    // Image views
    pub swapchain_image_views: Vec<vk::ImageView>,
    // Pipeline
    pub render_pass: vk::RenderPass,
    pub pipelines: Vec<GraphicsPipeline>,
    // Descriptors
    pub global_descriptor_set_layout: vk::DescriptorSetLayout,
    pub material_descriptor_set_layout: vk::DescriptorSetLayout,
    pub light_descriptor_set_layout: vk::DescriptorSetLayout,
    pub skybox_descriptor_set_layout: vk::DescriptorSetLayout,
    pub global_descriptor_sets: Vec<vk::DescriptorSet>,
    pub material_descriptor_sets: Vec<vk::DescriptorSet>,
    pub light_descriptor_sets: Vec<vk::DescriptorSet>,
    pub skybox_descriptor_sets: Vec<vk::DescriptorSet>,
    pub descriptor_pool: vk::DescriptorPool,
    // Framebuffers
    pub framebuffers: Vec<vk::Framebuffer>,
    // Command Pool
    pub command_pool: vk::CommandPool,
    // Color
    pub color_image: vk::Image,
    pub color_image_memory: vk::DeviceMemory,
    pub color_image_view: vk::ImageView,
    // Mesh/Object data for the next renderer step.
    pub meshes: Vec<Option<Mesh>>,
    pub render_objects: Vec<RenderItem>,
    pub text_render_objects: Vec<TextRenderItem>,
    pub skybox: Option<RenderSkybox>,
    // Buffers
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    pub light_uniform_buffers: Vec<vk::Buffer>,
    pub light_uniform_buffers_memory: Vec<vk::DeviceMemory>,
    // Command Buffers
    pub command_pools: Vec<vk::CommandPool>,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub secondary_command_buffers: Vec<Vec<vk::CommandBuffer>>,
    // Sync Objects
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    // Texture Image
    pub textures: Vec<Texture>,
    pub skybox_textures: Vec<Texture>,
    pub texture_sampler: vk::Sampler,
    // Depth
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
    // Camera
    pub camera: RenderCamera,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub vertex_buffer_size: vk::DeviceSize,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub index_buffer_size: vk::DeviceSize,

    pub index_count: u32,
    pub vertex_layout: VertexLayout,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    pub index: usize,
    pub vertex_layout: VertexLayout,
}

impl MeshHandle {
    pub const fn new(index: usize, vertex_layout: VertexLayout) -> Self {
        Self {
            index,
            vertex_layout,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Texture {
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
    pub mip_levels: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkyboxTextureHandle(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PipelineKey {
    Mesh3D,
    DebugLine3D,
    Transparent3D,
    Lit3D,
    Ui2D,
    Skybox,
    TextUi2D,
    Text3D,
}

impl PipelineKey {
    pub const fn required_vertex_layout(self) -> VertexLayout {
        match self {
            PipelineKey::Mesh3D => VertexLayout::Mesh3D,
            PipelineKey::Transparent3D => VertexLayout::Mesh3D,
            PipelineKey::DebugLine3D => VertexLayout::DebugLine3D,
            PipelineKey::Lit3D => VertexLayout::Lit3D,
            PipelineKey::Ui2D => VertexLayout::Ui2D,
            PipelineKey::Skybox => VertexLayout::Skybox,
            PipelineKey::TextUi2D => VertexLayout::Ui2D,
            PipelineKey::Text3D => VertexLayout::Ui2D,
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub struct GraphicsPipeline {
    pub key: PipelineKey,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,

    // vertex layout to use this pipeline
    pub vertex_layout: VertexLayout,
}

impl VulkanData {
    pub fn pipeline(&self, key: PipelineKey) -> &GraphicsPipeline {
        self.pipelines
            .iter()
            .find(|pipeline| pipeline.key == key)
            .expect("pipeline shoud exist")
    }
}

#[derive(Clone, Debug)]
pub struct RenderItem {
    pub mesh_index: MeshHandle,
    pub transform: Transform,
    pub alpha: f32,
    // material
    pub material_color: cgmath::Vector3<f32>,
    pub use_texture: bool,
    pub texture_index: TextureHandle, // use Texture from VulkanData::textures
    pub pipeline_key: PipelineKey,
    pub is_visible: bool,
}

#[derive(Clone,Debug)]
pub struct TextRenderItem{
    pub mesh: MeshHandle,
    pub atlas_page: usize,
    pub transform: Transform,
    pub alpha: f32,
    pub color: [f32;4],
    pub pipeline_key: PipelineKey,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RenderSkybox {
    pub mesh: MeshHandle,
    pub texture: SkyboxTextureHandle,
    pub is_visible: bool,
}

// Camera
#[derive(Clone, Debug)]
pub struct RenderCamera {
    pub position: cgmath::Vector3<f32>,
    pub target: cgmath::Vector3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for RenderCamera {
    fn default() -> Self {
        Self {
            position: vec3(0.0, 0.0, -5.0),
            target: vec3(0.0, 0.0, 0.0),
            up: vec3(0.0, 1.0, 0.0),
            fov_y: 0.0,
            near: 0.0,
            far: 0.0,
        }
    }
}

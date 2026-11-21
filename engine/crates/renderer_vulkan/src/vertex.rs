use anyhow::Result;
use std::hash::{Hash, Hasher};
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::ColorBlendEquationEXT;

use std::mem::size_of;

use super::buffer::{copy_buffer, create_buffer};
use super::types::VulkanData;

type Vec2 = cgmath::Vector2<f32>;
type Vec3 = cgmath::Vector3<f32>;


#[derive(Clone, Copy, Debug)]
pub struct SkinInfluence{
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SourceVertex {
    pub pos: Vec3,
    pub color: Vec3,
    pub tex_coord: Vec2, // points of a texture
    pub normal: Vec3,
    pub tangent: Option<[f32; 4]>,
    pub skin: Option<SkinInfluence>,
}

impl SourceVertex {
    pub fn new(
        pos: Vec3,
        color: Vec3, 
        tex_coord: Vec2, 
        normal: Vec3,
        tangent: Option<[f32; 4]>,
        skin: Option<SkinInfluence>,
    ) -> Self {
        Self {
            pos,
            color,
            tex_coord,
            normal,
            tangent, 
            skin,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexLayout {
    Mesh3D,
    DebugLine3D,
    Lit3D,
    Ui2D,
    Skybox,
    SkinnedVertex,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Mesh3DVertex {
    pub pos: Vec3,
    pub color: Vec3,
    pub tex_coord: Vec2,
}

impl From<&SourceVertex> for Mesh3DVertex {
    fn from(v: &SourceVertex) -> Self {
        Self {
            pos: v.pos,
            color: v.color,
            tex_coord: v.tex_coord,
        }
    }
}

impl Mesh3DVertex {
    pub const fn new(pos: Vec3, color: Vec3, tex_coord: Vec2) -> Self {
        Self {
            pos,
            color,
            tex_coord,
        }
    }
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Mesh3DVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();
        let tex_coord = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((size_of::<Vec3>() + size_of::<Vec3>()) as u32)
            .build();

        [pos, color, tex_coord]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DebugLineVertex {
    pub pos: Vec3,
    pub color: Vec3,
}

impl From<&SourceVertex> for DebugLineVertex {
    fn from(v: &SourceVertex) -> Self {
        Self {
            pos: v.pos,
            color: v.color,
        }
    }
}

impl DebugLineVertex {
    pub const fn new(pos: Vec3, color: Vec3) -> Self {
        Self { pos, color }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<DebugLineVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();

        [pos, color]
    }
}

impl PartialEq for SourceVertex {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
            && self.color == other.color
            && self.tex_coord == other.tex_coord
            && self.normal == other.normal
    }
}

impl Eq for SourceVertex {}

impl Hash for SourceVertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos[0].to_bits().hash(state);
        self.pos[1].to_bits().hash(state);
        self.pos[2].to_bits().hash(state);
        self.color[0].to_bits().hash(state);
        self.color[1].to_bits().hash(state);
        self.color[2].to_bits().hash(state);
        self.tex_coord[0].to_bits().hash(state);
        self.tex_coord[1].to_bits().hash(state);
        self.normal[0].to_bits().hash(state);
        self.normal[1].to_bits().hash(state);
        self.normal[2].to_bits().hash(state);
    }
}

// 1. record staging buffer
// 2. copy vertices from staging buffer to vertex buffer
pub unsafe fn create_vertex_buffer<V>(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    vertices: &[V],
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let size = std::mem::size_of_val(vertices) as u64;

    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;

    // memcpy(GPU_MEMORY,VERTICES,SIZE)
    memcpy(vertices.as_ptr(), memory.cast(), vertices.len());
    device.unmap_memory(staging_buffer_memory);

    let (vertex_buffer, vertex_buffer_memory) = create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    // copy staging buffer to vertex buffer
    copy_buffer(device, data, staging_buffer, vertex_buffer, size)?;

    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);
    Ok((vertex_buffer, vertex_buffer_memory))
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Lit3DVertex {
    pub pos: Vec3,
    pub color: Vec3,
    pub tex_coord: Vec2,
    pub normal: Vec3,
    pub tangent: [f32; 4],
}

impl From<&SourceVertex> for Lit3DVertex {
    fn from(v: &SourceVertex) -> Self {
        Self {
            pos: v.pos,
            color: v.color,
            tex_coord: v.tex_coord,
            normal: v.normal,
            tangent: v.tangent.unwrap_or([0.0; 4]),
        }
    }
}

impl Lit3DVertex {
    pub const fn new(pos: Vec3, color: Vec3, tex_coord: Vec2, normal: Vec3, tangent: Option<[f32; 4]>) -> Self {
        Self {
            pos,
            color,
            tex_coord,
            normal,
            tangent: match tangent { Some(value) => value, None => [0.0; 4] },
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Lit3DVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 5] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();

        let tex_coord = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((size_of::<Vec3>() + size_of::<Vec3>()) as u32)
            .build();

        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset((size_of::<Vec3>() + size_of::<Vec3>() + size_of::<Vec2>()) as u32)
            .build();
        let tangent= vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(4)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(std::mem::offset_of!(Lit3DVertex, tangent) as u32)
            .build();
        [pos, color, tex_coord, normal, tangent]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Ui2DVertex {
    pub pos: Vec2,
    pub color: Vec3,
    pub tex_coord: Vec2,
}

impl From<&SourceVertex> for Ui2DVertex {
    fn from(v: &SourceVertex) -> Self {
        Self {
            pos: cgmath::vec2(v.pos.y, v.pos.z),
            color: v.color,
            tex_coord: v.tex_coord,
        }
    }
}

impl Ui2DVertex {
    pub const fn new(pos: Vec2, color: Vec3, tex_coord: Vec2) -> Self {
        Self {
            pos,
            color,
            tex_coord,
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Ui2DVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0)
            .build();

        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec2>() as u32)
            .build();

        let tex_coord = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((size_of::<Vec2>() + size_of::<Vec3>()) as u32)
            .build();

        [pos, color, tex_coord]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SkyboxVertex {
    pub pos: Vec3,
}

impl From<&SourceVertex> for SkyboxVertex {
    fn from(v: &SourceVertex) -> Self {
        Self { pos: v.pos }
    }
}

impl SkyboxVertex {
    pub const fn new(pos: Vec3) -> Self {
        Self { pos }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<SkyboxVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 1] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        [pos]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SkinnedVertex{
    pub pos: Vec3,
    pub color: Vec3,
    pub tex_coord: Vec2,
    pub normal: Vec3,
    pub tangent: [f32; 4],
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

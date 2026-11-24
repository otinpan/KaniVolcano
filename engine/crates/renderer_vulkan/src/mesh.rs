use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

use super::index::create_index_buffer;
use super::model::MeshData;
use super::types::{Mesh, VulkanData};
use super::vertex::VertexLayout;
use super::vertex::create_vertex_buffer;

pub unsafe fn create_mesh<V>(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    mesh_data: MeshData<V>,
    vertex_layout: VertexLayout,
) -> Result<Mesh> {
    let index_count = mesh_data.indices.len() as u32;

    let vertex_buffer_size =
        (std::mem::size_of::<V>() * mesh_data.vertices.len()) as vk::DeviceSize;

    let index_buffer_size =
        (std::mem::size_of::<u32>() * mesh_data.indices.len()) as vk::DeviceSize;

    let (vertex_buffer, vertex_buffer_memory) =
        create_vertex_buffer(instance, device, data, &mesh_data.vertices)?;

    let (index_buffer, index_buffer_memory) =
        match create_index_buffer(instance, device, data, &mesh_data.indices) {
            Ok(buffers) => buffers,
            Err(error) => {
                // Vertex upload completed before the index upload started.
                device.destroy_buffer(vertex_buffer, None);
                device.free_memory(vertex_buffer_memory, None);
                return Err(error);
            }
        };

    Ok(Mesh {
        vertex_buffer,
        vertex_buffer_memory,
        vertex_buffer_size,
        index_buffer,
        index_buffer_memory,
        index_buffer_size,
        index_count,
        vertex_layout,
    })
}

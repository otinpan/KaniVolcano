use anyhow::Result;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

use super::buffer::{copy_buffer, create_buffer};
use super::types::VulkanData;

pub unsafe fn create_index_buffer(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    indices: &[u32],
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let size = (size_of::<u32>() * indices.len()) as u64;

    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    let mut destination = None;
    let result = (|| -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;

        memcpy(indices.as_ptr(), memory.cast(), indices.len());
        device.unmap_memory(staging_buffer_memory);

        let (index_buffer, index_buffer_memory) = create_buffer(
            instance,
            device,
            data,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        destination = Some((index_buffer, index_buffer_memory));
        copy_buffer(device, data, staging_buffer, index_buffer, size)?;

        Ok((index_buffer, index_buffer_memory))
    })();
    super::buffer::release_upload_buffer(device, data, staging_buffer, staging_buffer_memory);
    if result.is_err() {
        if let Some((buffer, memory)) = destination {
            super::buffer::release_upload_buffer(device, data, buffer, memory);
        }
    }
    result
}

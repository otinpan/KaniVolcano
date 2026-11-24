use anyhow::{Result, anyhow};
use vulkanalia::prelude::v1_0::*;

use super::types::VulkanData;

// Only used after an upload wait fails without proving completion or device loss.
#[derive(Clone, Debug, Default)]
pub struct DeferredUploadCleanup {
    pub pending: bool,
    pub buffers: Vec<(vk::Buffer, vk::DeviceMemory)>,
    pub images: Vec<(vk::Image, vk::DeviceMemory)>,
    pub commands: Vec<vk::CommandBuffer>,
}

pub unsafe fn release_upload_buffer(device: &Device, data: &VulkanData, buffer: vk::Buffer, memory: vk::DeviceMemory) {
    if data.deferred_upload_cleanup.borrow().pending {
        data.deferred_upload_cleanup.borrow_mut().buffers.push((buffer, memory));
    } else {
        device.destroy_buffer(buffer, None);
        device.free_memory(memory, None);
    }
}

pub unsafe fn release_upload_image(device: &Device, data: &VulkanData, image: vk::Image, memory: vk::DeviceMemory) {
    if data.deferred_upload_cleanup.borrow().pending {
        data.deferred_upload_cleanup.borrow_mut().images.push((image, memory));
    } else {
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
}

/// Caller must first establish device idle or device loss.
pub unsafe fn drain_upload_cleanup(device: &Device, data: &VulkanData) {
    let mut cleanup = data.deferred_upload_cleanup.borrow_mut();
    for (buffer, memory) in cleanup.buffers.drain(..) {
        device.destroy_buffer(buffer, None);
        device.free_memory(memory, None);
    }
    for (image, memory) in cleanup.images.drain(..) {
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
    if !cleanup.commands.is_empty() {
        device.free_command_buffers(data.command_pool, &cleanup.commands);
    }
    cleanup.commands.clear();
    cleanup.pending = false;
}

pub unsafe fn create_buffer(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = device.create_buffer(&buffer_info, None)?;

    let mut buffer_memory = vk::DeviceMemory::null();
    let result = (|| -> Result<()> {
        let requirements = device.get_buffer_memory_requirements(buffer);
        let memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(get_memory_type_index(
                instance,
                data,
                properties,
                requirements,
            )?);
        buffer_memory = device.allocate_memory(&memory_info, None)?;
        device.bind_buffer_memory(buffer, buffer_memory, 0)?;
        Ok(())
    })();
    if let Err(error) = result {
        device.destroy_buffer(buffer, None);
        device.free_memory(buffer_memory, None);
        return Err(error);
    }

    Ok((buffer, buffer_memory))
}

// copy source to destination
pub unsafe fn copy_buffer(
    device: &Device,
    data: &VulkanData,
    source: vk::Buffer,
    destination: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<()> {
    let command_buffer = begin_single_time_commands(device, data)?;

    // record instruction to submit to command buffer, but not copy yet.
    let regions = vk::BufferCopy::builder().size(size);
    device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);

    end_single_time_commands(device, data, command_buffer)?;
    Ok(())
}

pub unsafe fn begin_single_time_commands(
    device: &Device,
    data: &VulkanData,
) -> Result<vk::CommandBuffer> {
    // create command buffer
    let info = vk::CommandBufferAllocateInfo::builder()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(data.command_pool)
        .command_buffer_count(1);

    let command_buffer = device.allocate_command_buffers(&info)?[0];

    // start command buffer
    let info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    if let Err(error) = device.begin_command_buffer(command_buffer, &info) {
        device.free_command_buffers(data.command_pool, &[command_buffer]);
        return Err(error.into());
    }

    Ok(command_buffer)
}

pub unsafe fn end_single_time_commands(
    device: &Device,
    data: &VulkanData,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    if let Err(error) = device.end_command_buffer(command_buffer) {
        device.free_command_buffers(data.command_pool, &[command_buffer]);
        return Err(error.into());
    }

    let command_buffers = &[command_buffer];
    let info = vk::SubmitInfo::builder().command_buffers(command_buffers);

    // submit
    let result = device.queue_submit(data.graphics_queue, &[info], vk::Fence::null())
        .and_then(|_| device.queue_wait_idle(data.graphics_queue));
    if let Err(error) = result {
        match device.device_wait_idle() {
            Ok(()) | Err(vk::ErrorCode::DEVICE_LOST) => drain_upload_cleanup(device, data),
            Err(wait_error) => {
                let mut cleanup = data.deferred_upload_cleanup.borrow_mut();
                cleanup.pending = true;
                cleanup.commands.push(command_buffer);
                return Err(anyhow!(error).context(format!("Upload completion unknown; cleanup deferred: {wait_error}")));
            }
        }
        device.free_command_buffers(data.command_pool, &[command_buffer]);
        return Err(error.into());
    }

    device.free_command_buffers(data.command_pool, &[command_buffer]);

    Ok(())
}

unsafe fn get_memory_type_index(
    instance: &Instance,
    data: &VulkanData,
    properties: vk::MemoryPropertyFlags,
    requirements: vk::MemoryRequirements,
) -> Result<u32> {
    let memory = instance.get_physical_device_memory_properties(data.physical_device);

    (0..memory.memory_type_count)
        .find(|i| {
            let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
            let memory_type = memory.memory_types[*i as usize];
            suitable && memory_type.property_flags.contains(properties)
        })
        .ok_or_else(|| anyhow!("Failed to find sutable memory type."))
}

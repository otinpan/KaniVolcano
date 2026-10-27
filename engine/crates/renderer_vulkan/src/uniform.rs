use anyhow::Result;
use cgmath::InnerSpace;
use cgmath::{Deg, point3};
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

use super::VulkanRenderer;
use super::buffer::create_buffer;
use super::types::VulkanData;

type Mat4 = cgmath::Matrix4<f32>;

pub const MAX_POINT_LIGHTS: usize = 8;
pub const MAX_SPOT_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct UniformBufferObject {
    view: Mat4,
    proj: Mat4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct LightUniformBufferObject {
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub ambient: [f32; 4],

    pub light_params: [f32; 4], // x = spot light count, y = point light count

    pub point_light_positions: [[f32; 4]; MAX_POINT_LIGHTS],
    pub point_light_colors: [[f32; 4]; MAX_POINT_LIGHTS],

    pub spot_light_positions: [[f32; 4]; MAX_SPOT_LIGHTS],
    pub spot_light_directions: [[f32; 4]; MAX_SPOT_LIGHTS],
    pub spot_light_colors: [[f32; 4]; MAX_SPOT_LIGHTS],
    pub spot_light_cone_params: [[f32; 4]; MAX_SPOT_LIGHTS],
}

pub unsafe fn create_global_descriptor_set_layout(
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let bindings = &[ubo_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

    data.global_descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;

    Ok(())
}

pub unsafe fn create_pbr_descriptor_set_layout(device: &Device, data: &mut VulkanData) -> Result<()>{
    let bindings=[
        vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX),
        vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
    ];

    let info=vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
    data.pbr_global_descriptor_set_layout=device.create_descriptor_set_layout(&info, None)?;

    Ok(())
}

pub unsafe fn create_material_descriptor_set_layout(
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = &[sampler_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

    data.material_descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;

    Ok(())
}

pub unsafe fn create_light_descriptor_set_layout(
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let light_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = &[light_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

    data.light_descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;

    Ok(())
}

pub unsafe fn create_skybox_descriptor_set_layout(
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let skybox_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = &[skybox_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

    data.skybox_descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;

    Ok(())
}

// create pool to record descriptor_set
pub unsafe fn create_descriptor_pool(device: &Device, data: &mut VulkanData) -> Result<()> {
    // Global + light + PBR global (view/projection and camera position).
    let uniform_descriptor_count = data.swapchain_images.len() * 4;
    let skybox_descriptor_count = if data.skybox_textures.is_empty() {
        0
    } else {
        data.swapchain_images.len()
    };
    let max_sets = data.swapchain_images.len()
        + data.swapchain_images.len() // PBR global sets
        + data.textures.len()
        + data.swapchain_images.len()
        + skybox_descriptor_count;

    let ubo_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(uniform_descriptor_count as u32);

    let sampler_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count((data.textures.len() + skybox_descriptor_count) as u32);

    let pool_sizes = &[ubo_size, sampler_size];
    let info = vk::DescriptorPoolCreateInfo::builder()
        .pool_sizes(pool_sizes)
        .max_sets(max_sets as u32);

    data.descriptor_pool = device.create_descriptor_pool(&info, None)?;

    Ok(())
}

// if image_index is updated from render function,
// uniform_buffer[index] is updated
// and then reflect shader via descriptor sets which is binding with pipeline
// uniform buffer <-> descriptor set <-> pipeline layout <-> pipeline <-> shader
pub unsafe fn create_global_descriptor_sets(device: &Device, data: &mut VulkanData) -> Result<()> {
    let layouts = vec![data.global_descriptor_set_layout; data.swapchain_images.len()];

    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(data.descriptor_pool)
        .set_layouts(&layouts);

    data.global_descriptor_sets = device.allocate_descriptor_sets(&info)?;

    for i in 0..data.swapchain_images.len() {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(data.uniform_buffers[i])
            .offset(0)
            .range(size_of::<UniformBufferObject>() as u64);

        let buffer_infos = &[buffer_info];

        let ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(data.global_descriptor_sets[i])
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(buffer_infos);

        device.update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet]);
    }

    Ok(())
}

/// Requires an existing pool with room for one set and two UBO descriptors
/// per swapchain image, plus initialized view/projection and camera buffers.
pub unsafe fn create_pbr_global_descriptor_sets(
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let image_count = data.swapchain_images.len();
    anyhow::ensure!(image_count > 0, "PBR descriptor sets require swapchain images");
    anyhow::ensure!(
        data.uniform_buffers.len() == image_count
            && data.pbr_camera_uniform_buffers.len() == image_count,
        "PBR descriptor sets require view/projection and camera buffers for every swapchain image"
    );
    anyhow::ensure!(
        data.pbr_global_descriptor_set_layout != vk::DescriptorSetLayout::null()
            && data.descriptor_pool != vk::DescriptorPool::null(),
        "PBR descriptor layout and pool must be created before allocating sets"
    );
    anyhow::ensure!(
        data.uniform_buffers.iter().chain(&data.pbr_camera_uniform_buffers)
            .all(|buffer| *buffer != vk::Buffer::null()),
        "PBR descriptor sets cannot reference null uniform buffers"
    );

    let layouts = vec![data.pbr_global_descriptor_set_layout; image_count];
    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(data.descriptor_pool)
        .set_layouts(&layouts);
    let descriptor_sets = device.allocate_descriptor_sets(&info)?;

    for (i, &descriptor_set) in descriptor_sets.iter().enumerate() {
        let global_infos = [vk::DescriptorBufferInfo::builder()
            .buffer(data.uniform_buffers[i])
            .offset(0)
            .range(size_of::<UniformBufferObject>() as u64)];
        // CameraUniform.worldPosition is a GLSL vec4 (16 bytes).
        let camera_infos = [vk::DescriptorBufferInfo::builder()
            .buffer(data.pbr_camera_uniform_buffers[i])
            .offset(0)
            .range(size_of::<[f32; 4]>() as u64)];
        let writes = [
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&global_infos),
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&camera_infos),
        ];
        device.update_descriptor_sets(&writes, &[] as &[vk::CopyDescriptorSet]);
    }

    data.pbr_global_descriptor_sets = descriptor_sets;
    Ok(())
}

pub unsafe fn create_material_descriptor_sets(
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let layouts = vec![data.material_descriptor_set_layout; data.textures.len()];
    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(data.descriptor_pool)
        .set_layouts(&layouts);

    // allocate memory for descriptor set.
    data.material_descriptor_sets = device.allocate_descriptor_sets(&info)?;

    for i in 0..data.textures.len() {
        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(data.textures[i].image_view)
            .sampler(data.texture_sampler);

        let image_infos = &[image_info];

        let sampler_write = vk::WriteDescriptorSet::builder()
            .dst_set(data.material_descriptor_sets[i])
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(image_infos);

        device.update_descriptor_sets(&[sampler_write], &[] as &[vk::CopyDescriptorSet]);
    }

    Ok(())
}

pub unsafe fn create_light_descriptor_sets(device: &Device, data: &mut VulkanData) -> Result<()> {
    let layouts = vec![data.light_descriptor_set_layout; data.swapchain_images.len()];

    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(data.descriptor_pool)
        .set_layouts(&layouts);

    data.light_descriptor_sets = device.allocate_descriptor_sets(&info)?;

    for i in 0..data.swapchain_images.len() {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(data.light_uniform_buffers[i])
            .offset(0)
            .range(size_of::<LightUniformBufferObject>() as u64);

        let buffer_infos = &[buffer_info];

        let ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(data.light_descriptor_sets[i])
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(buffer_infos);

        device.update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet]);
    }

    Ok(())
}

pub unsafe fn create_skybox_descriptor_sets(device: &Device, data: &mut VulkanData) -> Result<()> {
    data.skybox_descriptor_sets.clear();

    if data.skybox_textures.is_empty() {
        return Ok(());
    }

    let layouts = vec![data.skybox_descriptor_set_layout; data.swapchain_images.len()];

    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(data.descriptor_pool)
        .set_layouts(&layouts);

    data.skybox_descriptor_sets = device.allocate_descriptor_sets(&info)?;

    for i in 0..data.swapchain_images.len() {
        let skybox_texture_index = data.skybox.map(|skybox| skybox.texture.0).unwrap_or(0);
        let skybox_texture = data.skybox_textures[skybox_texture_index];
        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(skybox_texture.image_view)
            .sampler(data.texture_sampler);

        let image_infos = &[image_info];

        let sampler_write = vk::WriteDescriptorSet::builder()
            .dst_set(data.skybox_descriptor_sets[i])
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(image_infos);

        device.update_descriptor_sets(&[sampler_write], &[] as &[vk::CopyDescriptorSet]);
    }

    Ok(())
}

pub unsafe fn create_uniform_buffers(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    data.uniform_buffers.clear();
    data.uniform_buffers_memory.clear();

    for _ in 0..data.swapchain_images.len() {
        let (uniform_buffer, uniform_buffer_memory) = create_buffer(
            instance,
            device,
            data,
            size_of::<UniformBufferObject>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        data.uniform_buffers.push(uniform_buffer);
        data.uniform_buffers_memory.push(uniform_buffer_memory);
    }

    Ok(())
}

pub unsafe fn update_uniform_buffer(
    renderer: &mut VulkanRenderer,
    image_index: usize,
) -> Result<()> {
    // crate camera
    let camera = &renderer.data.camera;

    let view = Mat4::look_at_rh(
        point3(camera.position.x, camera.position.y, camera.position.z),
        point3(camera.target.x, camera.target.y, camera.target.z),
        camera.up,
    );

    #[rustfmt::skip]
    let correction=Mat4::new(
        1.0,0.0,0.0,0.0,
        0.0,-1.0,0.0,0.0,
        0.0,0.0,1.0/2.0,0.0,
        0.0,0.0,1.0/2.0,1.0,
    );

    // how to project
    // when far from camera, then objects are shown smaller.
    let proj = correction
        * cgmath::perspective(
            Deg(camera.fov_y),
            renderer.data.swapchain_extent.width as f32
                / renderer.data.swapchain_extent.height as f32,
            camera.near,
            camera.far,
        );

    let ubo = UniformBufferObject { view, proj };

    // map gpu memory
    let memory = renderer.device.map_memory(
        renderer.data.uniform_buffers_memory[image_index],
        0,
        size_of::<UniformBufferObject>() as u64,
        vk::MemoryMapFlags::empty(),
    )?;

    // copy ubo to memory
    memcpy(&ubo, memory.cast(), 1);

    // unmap gpu memory
    renderer
        .device
        .unmap_memory(renderer.data.uniform_buffers_memory[image_index]);
    Ok(())
}

pub unsafe fn create_pbr_camera_uniform_buffers(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    anyhow::ensure!(
        data.pbr_camera_uniform_buffers.is_empty()
            && data.pbr_camera_uniform_buffers_memory.is_empty(),
        "PBR camera buffers must be destroyed before recreation"
    );
    for _ in 0..data.swapchain_images.len() {
        let allocation = create_buffer(
            instance,
            device,
            data,
            size_of::<[f32; 4]>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        );
        let (buffer, memory) = match allocation {
            Ok(allocation) => allocation,
            Err(error) => {
                for buffer in data.pbr_camera_uniform_buffers.drain(..) {
                    device.destroy_buffer(buffer, None);
                }
                for memory in data.pbr_camera_uniform_buffers_memory.drain(..) {
                    device.free_memory(memory, None);
                }
                return Err(error);
            }
        };
        data.pbr_camera_uniform_buffers.push(buffer);
        data.pbr_camera_uniform_buffers_memory.push(memory);
    }
    Ok(())
}

/// Call after waiting for this swapchain image's in-flight fence.
pub unsafe fn update_pbr_camera_uniform_buffer(
    renderer: &mut VulkanRenderer,
    image_index: usize,
) -> Result<()> {
    let position = renderer.data.camera.position;
    // Matches CameraUniform.worldPosition (vec4) in pbr3d.frag.
    let camera_data = [position.x, position.y, position.z, 1.0];
    let memory_handle = *renderer.data.pbr_camera_uniform_buffers_memory
        .get(image_index)
        .ok_or_else(|| anyhow::anyhow!("PBR camera buffer missing for image {image_index}"))?;
    let memory = renderer.device.map_memory(
        memory_handle,
        0,
        size_of::<[f32; 4]>() as u64,
        vk::MemoryMapFlags::empty(),
    )?;
    memcpy(camera_data.as_ptr(), memory.cast::<f32>(), camera_data.len());
    renderer.device.unmap_memory(memory_handle);
    Ok(())
}

pub unsafe fn create_light_uniform_buffers(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    data.light_uniform_buffers.clear();
    data.light_uniform_buffers_memory.clear();

    for _ in 0..data.swapchain_images.len() {
        let (light_uniform_buffer, light_uniform_buffer_memory) = create_buffer(
            instance,
            device,
            data,
            size_of::<LightUniformBufferObject>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        data.light_uniform_buffers.push(light_uniform_buffer);
        data.light_uniform_buffers_memory
            .push(light_uniform_buffer_memory);
    }

    Ok(())
}

pub unsafe fn update_light_uniform_buffer(
    renderer: &mut VulkanRenderer,
    image_index: usize,
) -> Result<()> {
    let mut point_light_positions = [[0.0; 4]; MAX_POINT_LIGHTS];
    let mut point_light_colors = [[0.0; 4]; MAX_POINT_LIGHTS];
    let mut spot_light_positions = [[0.0; 4]; MAX_SPOT_LIGHTS];
    let mut spot_light_directions = [[0.0; 4]; MAX_SPOT_LIGHTS];
    let mut spot_light_colors = [[0.0; 4]; MAX_SPOT_LIGHTS];
    let mut spot_light_cone_params = [[0.0; 4]; MAX_SPOT_LIGHTS];

    point_light_positions[0] = [-5.0, 0.0, 2.5, 6.0];
    point_light_colors[0] = [1.0, 0.85, 0.65, 2.0];

    point_light_positions[1] = [-5.0, 2.0, 1.5, 5.0];
    point_light_colors[1] = [0.35, 0.55, 1.0, 1.5];

    point_light_positions[2] = [-5.0, -2.0, 1.5, 5.0];
    point_light_colors[2] = [1.0, 0.35, 0.45, 1.5];

    let spot_light_pos = renderer.data.camera.position;
    let forward = renderer.data.camera.target - renderer.data.camera.position;
    let spot_light_dir = if forward.magnitude2() > f32::EPSILON {
        forward.normalize()
    } else {
        cgmath::vec3(1.0, 0.0, 0.0)
    };

    spot_light_positions[0] = [spot_light_pos.x, spot_light_pos.y, spot_light_pos.z, 12.0];

    spot_light_directions[0] = [spot_light_dir.x, spot_light_dir.y, spot_light_dir.z, 0.0];
    spot_light_colors[0] = [1.0, 1.0, 0.85, 3.0];
    spot_light_cone_params[0] = [
        15.0_f32.to_radians().cos(), //inner
        15.0_f32.to_radians().cos(), //outer
        0.0,
        0.0,
    ];

    let light = LightUniformBufferObject {
        direction: [-0.8, -1.0, -1.0, 0.0],
        color: [0.5, 0.5, 0.5, 1.0],
        ambient: [0.10, 0.10, 0.10, 1.0],
        light_params: [1.0, 3.0, 0.0, 0.0],
        point_light_positions,
        point_light_colors,
        spot_light_positions,
        spot_light_directions,
        spot_light_colors,
        spot_light_cone_params,
    };

    let memory = renderer.device.map_memory(
        renderer.data.light_uniform_buffers_memory[image_index],
        0,
        size_of::<LightUniformBufferObject>() as u64,
        vk::MemoryMapFlags::empty(),
    )?;

    memcpy(&light, memory.cast(), 1);

    renderer
        .device
        .unmap_memory(renderer.data.light_uniform_buffers_memory[image_index]);

    Ok(())
}

use anyhow::{Result, anyhow};
use std::fs::File;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

use super::Device;
use super::Instance;
use super::buffer::{begin_single_time_commands, create_buffer, end_single_time_commands};
use super::types::{Texture, VulkanData};

// texture //////////////////////////////////////////
pub unsafe fn create_texture(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
    path: &str,
) -> Result<Texture> {
    let image = File::open(path)?;

    let decoder = png::Decoder::new(image);
    let mut reader = decoder.read_info()?;

    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels)?;
    let pixels = &pixels[..info.buffer_size()];

    let pixels = match (reader.info().color_type, reader.info().bit_depth) {
        (png::ColorType::Rgb, png::BitDepth::Eight) => pixels
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect::<Vec<_>>(),
        (png::ColorType::Rgba, png::BitDepth::Eight) => pixels.to_vec(),
        (color_type, bit_depth) => {
            return Err(anyhow!(
                "Unsupported PNG format: {:?} {:?}",
                color_type,
                bit_depth,
            ));
        }
    };

    let (width, height) = reader.info().size();

    create_texture_from_pixels(instance, device, data, &pixels, width, height)
}

pub unsafe fn create_skybox_texture(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
    paths: [&str; 6],
) -> Result<Texture> {
    let mut pixels = Vec::new();
    let mut width = 0;
    let mut height = 0;

    for path in paths {
        let image = File::open(path)?;

        let decoder = png::Decoder::new(image);
        let mut reader = decoder.read_info()?;

        let mut face_pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut face_pixels)?;
        let face_pixels = &face_pixels[..info.buffer_size()];

        let face_pixels = match (reader.info().color_type, reader.info().bit_depth) {
            (png::ColorType::Rgb, png::BitDepth::Eight) => face_pixels
                .chunks_exact(3)
                .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                .collect::<Vec<_>>(),
            (png::ColorType::Rgba, png::BitDepth::Eight) => face_pixels.to_vec(),
            (color_type, bit_depth) => {
                return Err(anyhow!(
                    "Unsupported skybox PNG format: {:?} {:?}",
                    color_type,
                    bit_depth,
                ));
            }
        };

        let (face_width, face_height) = reader.info().size();
        if face_width != face_height {
            return Err(anyhow!(
                "Skybox face must be square: {} is {}x{}.",
                path,
                face_width,
                face_height,
            ));
        }

        if width == 0 {
            width = face_width;
            height = face_height;
        } else if width != face_width || height != face_height {
            return Err(anyhow!(
                "All skybox faces must have the same size. Expected {}x{}, got {}x{} for {}.",
                width,
                height,
                face_width,
                face_height,
                path,
            ));
        }

        pixels.extend_from_slice(&face_pixels);
    }

    create_cubemap_texture_from_pixels(instance, device, data, &pixels, width, height)
}

pub unsafe fn create_white_texture(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<Texture> {
    let pixels = [255, 255, 255, 255];

    create_texture_from_pixels(instance, device, data, &pixels, 1, 1)
}

pub unsafe fn create_black_skybox_texture(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<Texture> {
    let face = [0, 0, 0, 255];
    let mut pixels = Vec::with_capacity(face.len() * 6);

    for _ in 0..6 {
        pixels.extend_from_slice(&face);
    }

    create_cubemap_texture_from_pixels(instance, device, data, &pixels, 1, 1)
}

// texture image ///////////////////////////////////////////////
pub unsafe fn create_texture_from_pixels(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Texture> {
    let mip_levels = (width.max(height) as f32).log2().floor() as u32 + 1;
    let size = u64::try_from(pixels.len())?;
    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;
    memcpy(pixels.as_ptr(), memory.cast(), pixels.len());
    device.unmap_memory(staging_buffer_memory);

    let (image, image_memory) = create_image(
        instance,
        device,
        data,
        width,
        height,
        mip_levels,
        vk::SampleCountFlags::_1,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::SAMPLED
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    transition_image_layout(
        device,
        data,
        image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        mip_levels,
    )?;

    copy_buffer_to_image(device, data, staging_buffer, image, width, height)?;

    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);

    generate_mipmaps(
        instance,
        device,
        data,
        image,
        vk::Format::R8G8B8A8_SRGB,
        width,
        height,
        mip_levels,
    )?;

    let image_view = create_image_view(
        device,
        image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageAspectFlags::COLOR,
        mip_levels,
    )?;

    Ok(Texture {
        image,
        image_memory,
        image_view,
        mip_levels,
    })
}

pub unsafe fn create_mask_texture_from_pixels(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Texture>{
    anyhow::ensure!(
        width>0 && height>0,
        "mask texture size must be positive"
    );

    let expected_len=usize::try_from(width)?
        .checked_mul(usize::try_from(height)?)
        .ok_or_else(||anyhow!("mask texture size is too large"))?;

    anyhow::ensure!(
        pixels.len()==expected_len,
        "invalid mask texture data length"
    );

    let format=vk::Format::R8_UNORM;
    let mip_levels=1;
    let size=u64::try_from(pixels.len())?;

    let (staging_buffer,staging_buffer_memory)=create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    let memory=device.map_memory(staging_buffer_memory,0,size, vk::MemoryMapFlags::empty())?;
    memcpy(pixels.as_ptr(), memory.cast(), pixels.len());
    device.unmap_memory(staging_buffer_memory);

    let (image,image_memory) =create_image(
        instance,
        device,
        data,
        width,
        height,
        mip_levels,
        vk::SampleCountFlags::_1,
        format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    transition_image_layout(
        device,
        data,
        image,
        format,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        mip_levels,
    )?;

    copy_buffer_to_image(device,data,staging_buffer,image,width,height)?;

    transition_image_layout(
        device,
        data,
        image,
        format,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        mip_levels,
    )?;

    device.destroy_buffer(staging_buffer,None);
    device.free_memory(staging_buffer_memory, None);

    let image_view=create_image_view(
        device,
        image,
        format,
        vk::ImageAspectFlags::COLOR,
        mip_levels,
    )?;

    Ok(Texture { image, image_memory, image_view, mip_levels })
}



// update mask texture selecting gpu memory with "texture"
// send 1 page texture of atlas
pub unsafe fn update_mask_texture_from_pixels(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    texture: &Texture,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<()> {
    anyhow::ensure!(
        width > 0 && height > 0,
        "mask texture size must be positive"
    );

    anyhow::ensure!(
        texture.mip_levels == 1,
        "mask texture must have one mip level"
    );

    let expected_len = usize::try_from(width)?
        .checked_mul(usize::try_from(height)?)
        .ok_or_else(|| anyhow!("mask texture size is too large"))?;

    anyhow::ensure!(
        pixels.len() == expected_len,
        "invalid mask texture data length"
    );

    let size = u64::try_from(pixels.len())?;

    let (staging_buffer, staging_memory) = create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE
            | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let mapped = match device.map_memory(
        staging_memory,
        0,
        size,
        vk::MemoryMapFlags::empty(),
    ) {
        Ok(mapped) => mapped,
        Err(error) => {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
            return Err(error.into());
        }
    };

    memcpy(pixels.as_ptr(), mapped.cast(), pixels.len());
    device.unmap_memory(staging_memory);

    let result = (|| -> Result<()> {
        transition_image_layout(
            device,
            data,
            texture.image, // distination
            vk::Format::R8_UNORM,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            1,
        )?;

        copy_buffer_to_image(
            device,
            data,
            staging_buffer,
            texture.image,
            width,
            height,
        )?;

        transition_image_layout(
            device,
            data,
            texture.image,
            vk::Format::R8_UNORM,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            1,
        )?;

        Ok(())
    })();

    if result.is_err() {
        device.queue_wait_idle(data.graphics_queue)?;
    }

    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_memory, None);

    result
}


unsafe fn create_cubemap_texture_from_pixels(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Texture> {
    let mip_levels = 1;
    let layer_count = 6;
    let expected_size = width as usize * height as usize * 4 * layer_count;

    if pixels.len() != expected_size {
        return Err(anyhow!(
            "Skybox pixel data size mismatch. Expected {} bytes, got {} bytes.",
            expected_size,
            pixels.len(),
        ));
    }

    let size = pixels.len() as u64;

    let (staging_buffer, staging_buffer_memory) = create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;
    memcpy(pixels.as_ptr(), memory.cast(), pixels.len());
    device.unmap_memory(staging_buffer_memory);

    let (image, image_memory) = create_image_with_options(
        instance,
        device,
        data,
        width,
        height,
        mip_levels,
        layer_count as u32,
        vk::SampleCountFlags::_1,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        vk::ImageCreateFlags::CUBE_COMPATIBLE,
    )?;

    transition_image_layout_layers(
        device,
        data,
        image,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        mip_levels,
        layer_count as u32,
    )?;

    copy_buffer_to_image_layers(
        device,
        data,
        staging_buffer,
        image,
        width,
        height,
        layer_count as u32,
    )?;

    transition_image_layout_layers(
        device,
        data,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        mip_levels,
        layer_count as u32,
    )?;

    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);

    let image_view = create_image_view_with_options(
        device,
        image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageAspectFlags::COLOR,
        mip_levels,
        layer_count as u32,
        vk::ImageViewType::CUBE,
    )?;

    Ok(Texture {
        image,
        image_memory,
        image_view,
        mip_levels,
    })
}

unsafe fn create_image(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    width: u32,
    height: u32,
    mip_levels: u32,
    samples: vk::SampleCountFlags,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    create_image_with_options(
        instance,
        device,
        data,
        width,
        height,
        mip_levels,
        1,
        samples,
        format,
        tiling,
        usage,
        properties,
        vk::ImageCreateFlags::empty(),
    )
}

unsafe fn create_image_with_options(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    width: u32,
    height: u32,
    mip_levels: u32,
    array_layers: u32,
    samples: vk::SampleCountFlags,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
    flags: vk::ImageCreateFlags,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    // instraction what type of image
    let info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(mip_levels)
        .array_layers(array_layers)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(samples)
        .flags(flags);

    // create image, but not allocated in GPU memory
    let image = device.create_image(&info, None)?;
    // get memory size
    let requirements = device.get_image_memory_requirements(image);

    // search GPU memory which is DEVICE_LOCAL and usable this image
    let info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(get_memory_type_index(
            instance,
            data,
            properties,
            requirements,
        )?);

    // allocate
    let image_memory = device.allocate_memory(&info, None)?;

    // bind
    device.bind_image_memory(image, image_memory, 0)?;

    Ok((image, image_memory))
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
        .ok_or_else(|| anyhow!("Failed to find suitable memory type."))
}

// change image layout
unsafe fn transition_image_layout(
    device: &Device,
    data: &VulkanData,
    image: vk::Image,
    format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    mip_levels: u32,
) -> Result<()> {
    transition_image_layout_layers(device, data, image, old_layout, new_layout, mip_levels, 1)
}

unsafe fn transition_image_layout_layers(
    device: &Device,
    data: &VulkanData,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    mip_levels: u32,
    layer_count: u32,
) -> Result<()> {
    let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) =
        match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            (vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::ImageLayout::TRANSFER_DST_OPTIMAL) =>(
                vk::AccessFlags::SHADER_READ,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::PipelineStageFlags::TRANSFER,
            ),
            _ => return Err(anyhow!("Unsupported image layout transition!")),
        };

    let command_buffer = begin_single_time_commands(device, data)?;

    let subresource = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(mip_levels)
        .base_array_layer(0)
        .layer_count(layer_count);

    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource)
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask);

    device.cmd_pipeline_barrier(
        command_buffer,
        src_stage_mask,
        dst_stage_mask,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );

    end_single_time_commands(device, data, command_buffer)?;

    Ok(())
}

// copy image data in staging buffer to gpu
unsafe fn copy_buffer_to_image(
    device: &Device,
    data: &VulkanData,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
) -> Result<()> {
    copy_buffer_to_image_layers(device, data, buffer, image, width, height, 1)
}

unsafe fn copy_buffer_to_image_layers(
    device: &Device,
    data: &VulkanData,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
    layer_count: u32,
) -> Result<()> {
    // create command_buffer
    let command_buffer = begin_single_time_commands(device, data)?;

    let subresource = vk::ImageSubresourceLayers::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(layer_count);

    let region = vk::BufferImageCopy::builder()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(subresource)
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        });

    // write copy instruction to command_buffer
    // copy image from staging buffer to gpt image memory
    device.cmd_copy_buffer_to_image(
        command_buffer,
        buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[region],
    );

    end_single_time_commands(device, data, command_buffer)?;
    Ok(())
}

// tell vulkan how to use this image
// i.e. this image is 2D, this image is shown in this range
pub unsafe fn create_image_view(
    device: &Device,
    image: vk::Image,
    format: vk::Format,
    aspects: vk::ImageAspectFlags,
    mip_levels: u32,
) -> Result<vk::ImageView> {
    create_image_view_with_options(
        device,
        image,
        format,
        aspects,
        mip_levels,
        1,
        vk::ImageViewType::_2D,
    )
}

unsafe fn create_image_view_with_options(
    device: &Device,
    image: vk::Image,
    format: vk::Format,
    aspects: vk::ImageAspectFlags,
    mip_levels: u32,
    layer_count: u32,
    view_type: vk::ImageViewType,
) -> Result<vk::ImageView> {
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(aspects)
        .base_mip_level(0)
        .level_count(mip_levels)
        .base_array_layer(0)
        .layer_count(layer_count);

    let info = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(view_type)
        .format(format)
        .subresource_range(subresource_range);

    Ok(device.create_image_view(&info, None)?)
}

// sampler //////////////////////////////////////////////////
pub unsafe fn create_texture_sampler(device: &Device, data: &mut VulkanData) -> Result<()> {
    let info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT) // x
        .address_mode_v(vk::SamplerAddressMode::REPEAT) // y
        .address_mode_w(vk::SamplerAddressMode::REPEAT) // z
        .anisotropy_enable(true)
        .max_anisotropy(16.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(32.0)
        .mip_lod_bias(0.0);

    data.texture_sampler = device.create_sampler(&info, None)?;
    Ok(())
}

// depth objects /////////////////////////////////////////////////////////
pub unsafe fn create_depth_objects(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let format = get_depth_format(instance, data)?;

    // create empty depth image
    let (depth_image, depth_image_memory) = create_image(
        instance,
        device,
        data,
        data.swapchain_extent.width,
        data.swapchain_extent.height,
        1,
        data.msaa_samples,
        format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    data.depth_image = depth_image;
    data.depth_image_memory = depth_image_memory;

    // Image View
    data.depth_image_view = create_image_view(
        device,
        data.depth_image,
        format,
        vk::ImageAspectFlags::DEPTH,
        1,
    )?;

    Ok(())
}

pub unsafe fn get_depth_format(instance: &Instance, data: &VulkanData) -> Result<vk::Format> {
    let candidates = &[
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];

    get_supported_format(
        instance,
        data,
        candidates,
        vk::ImageTiling::OPTIMAL,
        vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
    )
}

// take a list of candidate formats in order from most desirable to least desirable
// and returns the first satisfies requirements
unsafe fn get_supported_format(
    instance: &Instance,
    data: &VulkanData,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Result<vk::Format> {
    candidates
        .iter()
        .cloned()
        .find(|f| {
            let properties =
                instance.get_physical_device_format_properties(data.physical_device, *f);
            match tiling {
                vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(features),
                vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(features),
                _ => false,
            }
        })
        .ok_or_else(|| anyhow!("Failed to find supported format!"))
}

// mipmap /////////////////////////////////////////////////////////
unsafe fn generate_mipmaps(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    image: vk::Image,
    format: vk::Format,
    width: u32,
    height: u32,
    mip_levels: u32,
) -> Result<()> {
    if !instance
        .get_physical_device_format_properties(data.physical_device, format)
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
    {
        return Err(anyhow!(
            "Texture image format does not support linear blitting!"
        ));
    }

    // start command buffer
    let command_buffer = begin_single_time_commands(device, data)?;

    let subresource = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_array_layer(0)
        .layer_count(1)
        .level_count(1);

    let mut barrier = vk::ImageMemoryBarrier::builder()
        .image(image)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .subresource_range(subresource);

    let mut mip_width = width;
    let mut mip_height = height;

    for i in 1..mip_levels {
        // invalid mipmap_level[i-1] state copy dst and copy src
        barrier.subresource_range.base_mip_level = i - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

        // shrink and copy mipmap_level[i-1] to mipmap_level[i]
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier],
        );
        let src_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(i - 1)
            .base_array_layer(0)
            .layer_count(1);

        let dst_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(i)
            .base_array_layer(0)
            .layer_count(1);

        let blit = vk::ImageBlit::builder()
            .src_offsets([
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: mip_width as i32,
                    y: mip_height as i32,
                    z: 1,
                },
            ])
            .src_subresource(src_subresource)
            .dst_offsets([
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: (if mip_width > 1 { mip_width / 2 } else { 1 }) as i32,
                    y: (if mip_height > 1 { mip_height / 2 } else { 1 }) as i32,
                    z: 1,
                },
            ])
            .dst_subresource(dst_subresource);

        // generate mipmap level from 1 texture incrementaly
        device.cmd_blit_image(
            command_buffer,
            image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[blit],
            vk::Filter::LINEAR,
        );

        // change mip_level texture from src_opt to shader readable
        barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier],
        );

        if mip_width > 1 {
            mip_width /= 2;
        }

        if mip_height > 1 {
            mip_height /= 2;
        }
    }

    barrier.subresource_range.base_mip_level = mip_levels - 1;
    barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
    barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
    barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
    barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

    device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );

    end_single_time_commands(device, data, command_buffer)?;
    Ok(())
}

// Color Object (MultiSampling) /////////////////////////////////////////////////////////
pub unsafe fn create_color_objects(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let (color_image, color_image_memory) = create_image(
        instance,
        device,
        data,
        data.swapchain_extent.width,
        data.swapchain_extent.height,
        1,
        data.msaa_samples,
        data.swapchain_format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    data.color_image = color_image;
    data.color_image_memory = color_image_memory;

    data.color_image_view = create_image_view(
        device,
        data.color_image,
        data.swapchain_format,
        vk::ImageAspectFlags::COLOR,
        1,
    )?;

    Ok(())
}

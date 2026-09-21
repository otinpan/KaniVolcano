use super::VulkanRenderer;
use super::device::QueueFamilyIndices;
use super::types::{PipelineKey, RenderSkybox, VulkanData};
use super::vertex::VertexLayout;
use anyhow::{Result,anyhow};
use cgmath::{Deg, vec3};
use vulkanalia::prelude::v1_0::*;

type Mat4 = cgmath::Matrix4<f32>;

#[repr(C)]
#[derive(Copy, Clone)]
struct FragmentPushConstants {
    material_color: [f32; 4],
    material_flags: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Ui2DTransformPushConstants {
    transform: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TextPushConstants {
    transform: [[f32; 4]; 4],
    color: [f32; 4],
    viewport_size: [f32; 2],
    padding: [f32; 2],
}

impl TextPushConstants {
    fn new(item: &super::types::TextRenderItem, extent: vk::Extent2D) -> Self {
        Self {
            transform: item.transform.matrix().into(),
            // Color alpha and overall opacity are multiplied independently.
            color: [item.color[0], item.color[1], item.color[2], item.color[3] * item.alpha],
            viewport_size: [extent.width as f32, extent.height as f32],
            padding: [0.0; 2],
        }
    }
}

#[cfg(test)]
mod text_tests {
    use super::*;

    #[test]
    fn push_constants_match_text_shader_offsets() {
        assert_eq!(std::mem::size_of::<TextPushConstants>(), 96);
        assert_eq!(std::mem::offset_of!(TextPushConstants, transform), 0);
        assert_eq!(std::mem::offset_of!(TextPushConstants, color), 64);
        assert_eq!(std::mem::offset_of!(TextPushConstants, viewport_size), 80);
        assert_eq!(std::mem::offset_of!(TextPushConstants, padding), 88);
    }

    #[test]
    fn push_constants_preserve_transform_and_combine_opacity() {
        let item = super::super::types::TextRenderItem {
            mesh: super::super::types::MeshHandle::new(0, VertexLayout::Ui2D),
            atlas_page: 0,
            transform: kani_volcano_math::Transform {
                position: vec3(20.0, 30.0, 0.0),
                rotation: vec3(0.0, 0.0, 45.0),
                scale: vec3(2.0, 3.0, 1.0),
            },
            alpha: 0.5,
            color: [1.0, 0.25, 0.75, 0.5],
        };
        let constants = TextPushConstants::new(&item, vk::Extent2D { width: 1280, height: 720 });
        let expected: [[f32; 4]; 4] = item.transform.matrix().into();
        assert_eq!(constants.transform, expected);
        assert_eq!(constants.color, [1.0, 0.25, 0.75, 0.25]);
        assert_eq!(constants.viewport_size, [1280.0, 720.0]);
        assert_eq!(constants.padding, [0.0; 2]);
    }
}

// command pool ////////////////////////////////////////////////////////////
// created command buffers are pushed into graphics queue in render()
// this command buffer is created at once
pub unsafe fn create_command_pools(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    data.command_pool = create_command_pool(instance, device, data)?;

    let num_images = data.swapchain_images.len();
    for _ in 0..num_images {
        let command_pool = create_command_pool(instance, device, data)?;
        data.command_pools.push(command_pool);
    }
    Ok(())
}

unsafe fn create_command_pool(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<vk::CommandPool> {
    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

    let info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::TRANSIENT)
        .queue_family_index(indices.graphics);

    Ok(device.create_command_pool(&info, None)?)
}

// command buffer /////////////////////////////////////////////////////////////
pub unsafe fn create_command_buffers(device: &Device, data: &mut VulkanData) -> Result<()> {
    let num_images = data.swapchain_images.len();
    for image_index in 0..num_images {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(data.command_pools[image_index])
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = device.allocate_command_buffers(&allocate_info)?[0];
        data.command_buffers.push(command_buffer);
    }

    data.secondary_command_buffers = vec![vec![]; data.swapchain_images.len()];
    Ok(())
}

// update command_buffer per frame
pub unsafe fn update_command_buffer(
    renderer: &mut VulkanRenderer,
    image_index: usize,
) -> Result<()> {
    // free memory to avoid memory leak
    let command_pool = renderer.data.command_pools[image_index];
    renderer
        .device
        .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())?;
    // recreate command_buffer
    let command_buffer = renderer.data.command_buffers[image_index];

    // Commands
    let info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    renderer
        .device
        .begin_command_buffer(command_buffer, &info)?;

    let render_area = vk::Rect2D::builder()
        .offset(vk::Offset2D::default())
        .extent(renderer.data.swapchain_extent);

    let color_clear_value = vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    };

    let depth_clear_value = vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        },
    };

    let clear_values = &[color_clear_value, depth_clear_value];

    let info = vk::RenderPassBeginInfo::builder()
        .render_pass(renderer.data.render_pass)
        .framebuffer(renderer.data.framebuffers[image_index])
        .render_area(render_area)
        .clear_values(clear_values);

    renderer.device.cmd_begin_render_pass(
        command_buffer,
        &info,
        vk::SubpassContents::SECONDARY_COMMAND_BUFFERS,
    );

    let mut secondary_command_buffers = Vec::new();

    if let Some(skybox) = renderer.data.skybox {
        if skybox.is_visible
            && renderer.data.skybox_descriptor_sets.len() > image_index
            && renderer
                .data
                .meshes
                .get(skybox.mesh.index)
                .is_some_and(|slot| slot.is_some())
        {
            secondary_command_buffers.push(update_skybox_command_buffer(
                renderer,
                image_index,
                skybox,
            )?);
        }
    }

    let visible_indices = sorted_render_indices(&renderer.data);
    secondary_command_buffers.extend(
        visible_indices
            .into_iter()
            .map(|i| update_secondary_command_buffer(renderer, image_index, i))
            .collect::<Result<Vec<_>, _>>()?,
    );

    if !renderer.data.text_render_objects.is_empty() {
        secondary_command_buffers.push(text_command_buffer(renderer, image_index)?);
    }

    if !secondary_command_buffers.is_empty() {
        renderer
            .device
            .cmd_execute_commands(command_buffer, &secondary_command_buffers[..]);
    }
    renderer.device.cmd_end_render_pass(command_buffer);

    renderer.device.end_command_buffer(command_buffer)?;

    Ok(())
}

// Record UI text after ordinary render items, preserving text batch order.
unsafe fn text_command_buffer(
    renderer: &mut VulkanRenderer,
    image_index: usize,
) -> Result<vk::CommandBuffer> {
    let extent = renderer.data.swapchain_extent;
    anyhow::ensure!(extent.width > 0 && extent.height > 0, "text viewport is empty");
    let pipeline = renderer.data.pipelines.iter()
        .find(|pipeline| pipeline.key == PipelineKey::TextUi2D)
        .copied()
        .ok_or_else(|| anyhow!("TextUi2D pipeline not found"))?;

    // Resolve resources before beginning the secondary command buffer.
    let draws = renderer.data.text_render_objects.iter().map(|item| {
        let mesh = renderer.data.meshes.get(item.mesh.index)
            .and_then(|slot| slot.as_ref())
            .ok_or_else(|| anyhow!("text mesh not found: {}", item.mesh.index))?;
        anyhow::ensure!(
            item.mesh.vertex_layout == VertexLayout::Ui2D
                && mesh.vertex_layout == VertexLayout::Ui2D,
            "text mesh must use Ui2D vertices"
        );
        let descriptor = renderer.gpu_glyph_atlas.descriptor_set(item.atlas_page)
            .ok_or_else(|| anyhow!("atlas descriptor not found: {}", item.atlas_page))?;
        Ok((mesh.vertex_buffer, mesh.index_buffer, mesh.index_count,
            descriptor, TextPushConstants::new(item, extent)))
    }).collect::<Result<Vec<_>>>()?;

    // Slot 0 is the skybox; ordinary objects use model_index + 1.
    let slot = renderer.data.render_objects.len() + 1;
    renderer.data.secondary_command_buffers.resize_with(image_index + 1, Vec::new);
    let buffers = &mut renderer.data.secondary_command_buffers[image_index];
    while buffers.len() <= slot {
        let allocation = vk::CommandBufferAllocateInfo::builder()
            .command_pool(renderer.data.command_pools[image_index])
            .level(vk::CommandBufferLevel::SECONDARY)
            .command_buffer_count(1);
        buffers.push(renderer.device.allocate_command_buffers(&allocation)?[0]);
    }
    let command_buffer = buffers[slot];
    let inheritance = vk::CommandBufferInheritanceInfo::builder()
        .render_pass(renderer.data.render_pass)
        .subpass(0)
        .framebuffer(renderer.data.framebuffers[image_index]);
    let begin = vk::CommandBufferBeginInfo::builder()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT
            | vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE)
        .inheritance_info(&inheritance);
    renderer.device.begin_command_buffer(command_buffer, &begin)?;
    renderer.device.cmd_bind_pipeline(command_buffer,
        vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);

    for (vertex_buffer, index_buffer, index_count, descriptor, constants) in draws {
        renderer.device.cmd_bind_descriptor_sets(command_buffer,
            vk::PipelineBindPoint::GRAPHICS, pipeline.layout, 0, &[descriptor], &[]);
        renderer.device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
        renderer.device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
        let bytes = std::slice::from_raw_parts(
            &constants as *const TextPushConstants as *const u8,
            std::mem::size_of::<TextPushConstants>(),
        );
        renderer.device.cmd_push_constants(command_buffer, pipeline.layout,
            vk::ShaderStageFlags::VERTEX, 0, bytes);
        renderer.device.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 0);
    }
    renderer.device.end_command_buffer(command_buffer)?;
    Ok(command_buffer)
}

unsafe fn update_skybox_command_buffer(
    renderer: &mut VulkanRenderer,
    image_index: usize,
    skybox: RenderSkybox,
) -> Result<vk::CommandBuffer> {
    renderer
        .data
        .secondary_command_buffers
        .resize_with(image_index + 1, Vec::new);

    let command_buffer = {
        let command_buffers = &mut renderer.data.secondary_command_buffers[image_index];

        while command_buffers.is_empty() {
            let allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(renderer.data.command_pools[image_index])
                .level(vk::CommandBufferLevel::SECONDARY)
                .command_buffer_count(1);

            let command_buffer = renderer.device.allocate_command_buffers(&allocate_info)?[0];
            command_buffers.push(command_buffer);
        }

        command_buffers[0]
    };

    let pipeline = renderer.data.pipeline(PipelineKey::Skybox);
    let Some(mesh) = renderer
        .data
        .meshes
        .get(skybox.mesh.index)
        .and_then(|slot| slot.as_ref())
    else {
        return Ok(command_buffer);
    };

    let inheritance_info = vk::CommandBufferInheritanceInfo::builder()
        .render_pass(renderer.data.render_pass)
        .subpass(0)
        .framebuffer(renderer.data.framebuffers[image_index]);

    let info = vk::CommandBufferBeginInfo::builder()
        .flags(vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE)
        .inheritance_info(&inheritance_info);

    renderer
        .device
        .begin_command_buffer(command_buffer, &info)?;

    if mesh.vertex_layout != pipeline.vertex_layout {
        renderer.device.end_command_buffer(command_buffer)?;
        return Ok(command_buffer);
    }

    renderer.device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline,
    );
    renderer
        .device
        .cmd_bind_vertex_buffers(command_buffer, 0, &[mesh.vertex_buffer], &[0]);
    renderer.device.cmd_bind_index_buffer(
        command_buffer,
        mesh.index_buffer,
        0,
        vk::IndexType::UINT32,
    );

    let global_set = renderer.data.global_descriptor_sets[image_index];
    let skybox_set = renderer.data.skybox_descriptor_sets[image_index];
    let sets = [global_set, skybox_set];

    renderer.device.cmd_bind_descriptor_sets(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.layout,
        0,
        &sets,
        &[],
    );

    renderer
        .device
        .cmd_draw_indexed(command_buffer, mesh.index_count, 1, 0, 0, 0);

    renderer.device.end_command_buffer(command_buffer)?;

    Ok(command_buffer)
}

// sort in o
fn sorted_render_indices(data: &VulkanData) -> Vec<usize> {
    let mut opaque_indices = Vec::new();
    let mut transparent_indices = Vec::new();
    let mut ui_indices = Vec::new();

    for (index, object) in data.render_objects.iter().enumerate() {
        if !object.is_visible {
            continue;
        }
        if data
            .meshes
            .get(object.mesh_index.index)
            .is_none_or(|slot| slot.is_none())
        {
            continue;
        }

        match object.pipeline_key {
            PipelineKey::Skybox => continue,
            PipelineKey::Transparent3D => transparent_indices.push(index),
            PipelineKey::Ui2D => ui_indices.push(index),
            _ => opaque_indices.push(index),
        }
    }

    // order near is faster
    let camera_position = data.camera.position;
    transparent_indices.sort_by(|&a, &b| {
        let distance_a =
            distance_squared(data.render_objects[a].transform.position, camera_position);
        let distance_b =
            distance_squared(data.render_objects[b].transform.position, camera_position);

        distance_b
            .partial_cmp(&distance_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut sorted_indices = opaque_indices;
    sorted_indices.extend(transparent_indices);
    sorted_indices.extend(ui_indices);
    sorted_indices
}

fn distance_squared(a: cgmath::Vector3<f32>, b: cgmath::Vector3<f32>) -> f32 {
    let d = a - b;
    d.x * d.x + d.y * d.y + d.z * d.z
}

unsafe fn update_secondary_command_buffer(
    renderer: &mut VulkanRenderer,
    image_index: usize,
    model_index: usize,
) -> Result<vk::CommandBuffer> {
    // if secondary_command_buffer (swapchain_images.len())
    // is smaller than index, add new vec
    renderer
        .data
        .secondary_command_buffers
        .resize_with(image_index + 1, Vec::new);

    let command_buffer = {
        let command_buffers = &mut renderer.data.secondary_command_buffers[image_index];

        let command_slot = model_index + 1;

        while command_slot >= command_buffers.len() {
            let allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(renderer.data.command_pools[image_index])
                .level(vk::CommandBufferLevel::SECONDARY)
                .command_buffer_count(1);

            let command_buffer = renderer.device.allocate_command_buffers(&allocate_info)?[0];
            command_buffers.push(command_buffer);
        }

        command_buffers[command_slot]
    };

    //  Model
    let object = &renderer.data.render_objects[model_index];
    if object.pipeline_key == PipelineKey::Skybox {
        return Ok(command_buffer);
    }

    let pipeline = renderer.data.pipeline(object.pipeline_key);
    let Some(mesh) = renderer
        .data
        .meshes
        .get(object.mesh_index.index)
        .and_then(|slot| slot.as_ref())
    else {
        return Ok(command_buffer);
    };

    let model = object.transform.matrix();

    let model_bytes =
        std::slice::from_raw_parts(&model as *const Mat4 as *const u8, size_of::<Mat4>());

    let material = FragmentPushConstants {
        material_color: [
            object.material_color.x,
            object.material_color.y,
            object.material_color.z,
            object.alpha,
        ],
        material_flags: [if object.use_texture { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
    };

    let material_bytes = std::slice::from_raw_parts(
        &material as *const FragmentPushConstants as *const u8,
        std::mem::size_of::<FragmentPushConstants>(),
    );

    // Unique Info for Secondary Command buffeer
    let inheritance_info = vk::CommandBufferInheritanceInfo::builder()
        .render_pass(renderer.data.render_pass)
        .subpass(0)
        .framebuffer(renderer.data.framebuffers[image_index]);

    let info = vk::CommandBufferBeginInfo::builder()
        .flags(vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE)
        .inheritance_info(&inheritance_info);

    // Start command buffer
    renderer
        .device
        .begin_command_buffer(command_buffer, &info)?;

    debug_assert_eq!(mesh.vertex_layout, pipeline.vertex_layout);

    if mesh.vertex_layout != pipeline.vertex_layout {
        renderer.device.end_command_buffer(command_buffer)?;
        return Ok(command_buffer);
    }

    renderer.device.cmd_bind_pipeline(
        command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        pipeline.pipeline,
    );
    renderer
        .device
        .cmd_bind_vertex_buffers(command_buffer, 0, &[mesh.vertex_buffer], &[0]);
    renderer.device.cmd_bind_index_buffer(
        command_buffer,
        mesh.index_buffer,
        0,
        vk::IndexType::UINT32,
    );

    match pipeline.key {
        PipelineKey::Mesh3D => {
            let global_set = renderer.data.global_descriptor_sets[image_index];
            let material_set = renderer.data.material_descriptor_sets[object.texture_index.0];
            let sets = [global_set, material_set];

            renderer.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                &sets,
                &[],
            );

            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                model_bytes,
            );
            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::FRAGMENT,
                64,
                material_bytes,
            );
        }
        PipelineKey::DebugLine3D => {
            let global_set = renderer.data.global_descriptor_sets[image_index];
            let sets = [global_set];

            renderer.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                &sets,
                &[],
            );

            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                model_bytes,
            );
        }
        PipelineKey::Transparent3D => {
            let global_set = renderer.data.global_descriptor_sets[image_index];
            let material_set = renderer.data.material_descriptor_sets[object.texture_index.0];
            let sets = [global_set, material_set];

            renderer.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                &sets,
                &[],
            );

            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                model_bytes,
            );
            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::FRAGMENT,
                64,
                material_bytes,
            );
        }
        PipelineKey::Lit3D => {
            let global_set = renderer.data.global_descriptor_sets[image_index];
            let material_set = renderer.data.material_descriptor_sets[object.texture_index.0];
            let light_set = renderer.data.light_descriptor_sets[image_index];

            let sets = [global_set, material_set, light_set];

            renderer.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                &sets,
                &[],
            );

            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                model_bytes,
            );
            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::FRAGMENT,
                64,
                material_bytes,
            );
        }
        PipelineKey::Ui2D => {
            let material_set = renderer.data.material_descriptor_sets[object.texture_index.0];
            let sets = [material_set];
            let ui_transform = Ui2DTransformPushConstants {
                transform: [
                    object.transform.position.y,
                    object.transform.position.z,
                    object.transform.scale.y,
                    object.transform.scale.z,
                ],
            };
            let ui_transform_bytes = std::slice::from_raw_parts(
                &ui_transform as *const Ui2DTransformPushConstants as *const u8,
                std::mem::size_of::<Ui2DTransformPushConstants>(),
            );

            renderer.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                &sets,
                &[],
            );

            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                ui_transform_bytes,
            );

            renderer.device.cmd_push_constants(
                command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::FRAGMENT,
                16,
                material_bytes,
            );
        }
        PipelineKey::TextUi2D =>{
            renderer.device.end_command_buffer(command_buffer)?;
            return Ok(command_buffer);
        }
        PipelineKey::Skybox => {
            renderer.device.end_command_buffer(command_buffer)?;
            return Ok(command_buffer);
        }
    }

    renderer
        .device
        .cmd_draw_indexed(command_buffer, mesh.index_count, 1, 0, 0, 0);

    renderer.device.end_command_buffer(command_buffer)?;

    Ok(command_buffer)
}

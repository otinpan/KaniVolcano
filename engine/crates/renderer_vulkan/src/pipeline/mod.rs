// pipeline is like a cartridge
// each pipeline has single vertex shader
// you can create pipelines for UI(2D) and for Actor(3D), and so on.
// when you want to switch render format, you switch pipelint like cartridges.
use anyhow::Result;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::prelude::v1_0::*;

mod mesh3d;
mod lit3d;
mod debug_line;
mod transparent3d;
mod ui2d;
mod text_ui2d;
mod text3d;
mod skybox;
mod pbr3d;

pub use mesh3d::{create_mesh3d_pipeline};
pub use lit3d::{create_lit3d_pipeline};
pub use debug_line::{create_debug_line_pipeline};
pub use transparent3d::{create_transparent3d_pipeline};
pub use ui2d::{create_ui2d_pipeline};
pub use text_ui2d::{create_text_ui2d_pipeline};
pub use text3d::{create_text_3d_pipeline};
pub use skybox::{create_skybox_pipeline};
pub use pbr3d::{create_pbr3d_pipeline};

use super::image::get_depth_format;
use super::types::{GraphicsPipeline, PipelineKey, VulkanData};
use super::vertex::{
    DebugLineVertex, Lit3DVertex, Mesh3DVertex, SkyboxVertex, Ui2DVertex, VertexLayout,
};

// A graphics pipeline describes how vertices and fragments
// are processed by the GPU.
// create "pipeline" between shader and engine
unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    // convert u8 to u32
    let bytecode = Bytecode::new(bytecode).unwrap();
    let info = vk::ShaderModuleCreateInfo::builder()
        .code(bytecode.code())
        .code_size(bytecode.code_size());
    Ok(device.create_shader_module(&info, None)?)
}

// A render pass describes the rendering operations and the attachments that will be used
pub unsafe fn create_render_pass(
    instance: &Instance,
    device: &Device,
    data: &mut VulkanData,
) -> Result<()> {
    let multisampled = data.msaa_samples != vk::SampleCountFlags::_1;
    let color_attachment = vk::AttachmentDescription::builder()
        .format(data.swapchain_format)
        .samples(data.msaa_samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(if multisampled {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        } else {
            vk::ImageLayout::PRESENT_SRC_KHR
        });

    let depth_stencil_attachment = vk::AttachmentDescription::builder()
        .format(get_depth_format(instance, data)?)
        .samples(data.msaa_samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_resolve_attachment = vk::AttachmentDescription::builder()
        .format(data.swapchain_format)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    // subpass
    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let depth_stencil_attachment_ref = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_resolve_attachment_ref = vk::AttachmentReference::builder()
        .attachment(2)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = &[color_attachment_ref];
    let resolve_attachments = &[color_resolve_attachment_ref];
    let mut subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments)
        .depth_stencil_attachment(&depth_stencil_attachment_ref);
    if multisampled {
        subpass = subpass.resolve_attachments(resolve_attachments);
    }

    // Dependencies
    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        );

    // Create
    let attachments = [
        color_attachment,
        depth_stencil_attachment,
        color_resolve_attachment,
    ];
    let subpasses = &[subpass];
    let dependencies = &[dependency];
    let info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments[..if multisampled { 3 } else { 2 }])
        .subpasses(subpasses)
        .dependencies(dependencies);

    data.render_pass = device.create_render_pass(&info, None)?;

    Ok(())
}

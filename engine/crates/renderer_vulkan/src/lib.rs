#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_imports,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod buffer;
mod command;
mod device;
mod image;
mod index;
mod instance;
mod mesh;
mod model;
mod pipeline;
mod swapchain;
mod sync;
mod types;
mod uniform;
mod vertex;
mod text;

use anyhow::{Result, anyhow};
use cgmath::Matrix4;
use kani_volcano_math::Transform;
use kani_volcano_text::{LoadedFont, GlyphAtlas, TextSystem};
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::ExtDebugUtilsExtensionInstanceCommands;
use vulkanalia::vk::KhrSurfaceExtensionInstanceCommands;
use vulkanalia::vk::KhrSwapchainExtensionDeviceCommands;
use vulkanalia::window as vk_window;
use winit::window::Window;

use self::buffer::{copy_buffer, create_buffer};
use self::command::{create_command_buffers, create_command_pools, update_command_buffer};
use self::device::{create_logical_device, pick_physical_device};
use self::image::{
    create_black_skybox_texture, create_color_objects, create_depth_objects, create_skybox_texture,
    create_texture, create_texture_sampler, create_white_texture,
};
use self::instance::{VALIDATION_ENABLED, create_entry, create_instance};
use self::mesh::create_mesh;
use self::model::{MeshData, load_model_source};
pub use self::model::{SourceMesh, SourceTopology};
use self::pipeline::{
    create_debug_line_pipeline, create_lit3d_pipeline, create_mesh3d_pipeline, create_render_pass,
    create_skybox_pipeline, create_transparent3d_pipeline, create_ui2d_pipeline, create_text_ui2d_pipeline,
};
use self::swapchain::{create_framebuffers, create_swapchain, create_swapchain_image_views};
use self::sync::{create_render_finished_semaphores, create_sync_objects};
use self::types::{Mesh, RenderSkybox, VulkanData};
pub use self::types::{
    MeshHandle, PipelineKey, RenderCamera, RenderItem, SkyboxTextureHandle, TextureHandle,
};
use self::uniform::{
    create_descriptor_pool, create_global_descriptor_set_layout, create_global_descriptor_sets,
    create_light_descriptor_set_layout, create_light_descriptor_sets, create_light_uniform_buffers,
    create_material_descriptor_set_layout, create_material_descriptor_sets,
    create_skybox_descriptor_set_layout, create_skybox_descriptor_sets, create_uniform_buffers,
    update_light_uniform_buffer, update_uniform_buffer,
};
pub use self::vertex::{DebugLineVertex, Mesh3DVertex, SourceVertex, VertexLayout};
use self::text::{GpuGlyphAtlas};
pub use self::text::{GpuTextBatch, GpuTextMesh};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
type Mat4 = Matrix4<f32>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontHandle(pub usize);

pub struct VulkanRenderer {
    entry: Entry,
    instance: Instance,
    pub data: VulkanData,
    device: Device,
    frame: usize,
    pub resized: bool,

    // text
    text_system: TextSystem,
    glyph_atlas: GlyphAtlas,
    gpu_glyph_atlas: GpuGlyphAtlas,

    fonts: Vec<LoadedFont>,
}

impl VulkanRenderer {
    pub unsafe fn create(window: &Window) -> Result<Self> {
        let entry = create_entry()?;
        let mut data = VulkanData::default();
        let instance = create_instance(window, &entry, &mut data)?;
        data.surface = vk_window::create_surface(&instance, &window, &window)?;
        let text_system=TextSystem::default();
        let glyph_atlas=GlyphAtlas::new(1024,1024)?;
        let gpu_glyph_atlas=GpuGlyphAtlas::new();
        // device
        pick_physical_device(&instance, &mut data)?;
        let device = create_logical_device(&entry, &instance, &mut data)?;
        create_swapchain(window, &instance, &device, &mut data)?;
        create_swapchain_image_views(&device, &mut data)?;
        create_render_pass(&instance, &device, &mut data)?;
        create_global_descriptor_set_layout(&device, &mut data)?;
        create_material_descriptor_set_layout(&device, &mut data)?;
        create_light_descriptor_set_layout(&device, &mut data)?;
        create_skybox_descriptor_set_layout(&device, &mut data)?;
        create_pipelines(&device, &mut data)?;
        create_command_pools(&instance, &device, &mut data)?;
        create_color_objects(&instance, &device, &mut data)?;
        create_depth_objects(&instance, &device, &mut data)?;
        create_framebuffers(&device, &mut data)?;
        create_texture_sampler(&device, &mut data)?;
        let white_texture = create_white_texture(&instance, &device, &mut data)?;
        data.textures.push(white_texture);
        let black_skybox_texture = create_black_skybox_texture(&instance, &device, &mut data)?;
        data.skybox_textures.push(black_skybox_texture);
        create_uniform_buffers(&instance, &device, &mut data)?;
        create_light_uniform_buffers(&instance, &device, &mut data)?;
        create_descriptor_pool(&device, &mut data)?;
        create_global_descriptor_sets(&device, &mut data)?;
        create_material_descriptor_sets(&device, &mut data)?;
        create_light_descriptor_sets(&device, &mut data)?;
        create_skybox_descriptor_sets(&device, &mut data)?;
        create_command_buffers(&device, &mut data)?;
        create_sync_objects(&device, &mut data)?;
        Ok(Self {
            entry,
            instance,
            data,
            device,
            frame: 0,
            resized: false,
            text_system,
            glyph_atlas,
            gpu_glyph_atlas,
            fonts: Vec::new(),
        })
    }

    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        // 1. wait for fence
        let in_flight_fence = self.data.in_flight_fences[self.frame];
        self.device
            .wait_for_fences(&[in_flight_fence], true, u64::MAX)?;

        // 2. When get swapchain image, then signal image_available_semaphore
        let result = self.device.acquire_next_image_khr(
            self.data.swapchain,
            u64::MAX,
            self.data.image_available_semaphores[self.frame],
            vk::Fence::null(),
        );

        // 3. if swapchain is invalid because changed window size,
        // recreate swapchain
        let (image_index, acquire_code) = match result {
            Ok((image_index, code)) => (image_index as usize, code),
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => return self.recreate_swapchain(window),
            Err(e) => return Err(anyhow!(e)),
        };

        // 4. if swapchain image is used wait fence
        let image_in_flight = self.data.images_in_flight[image_index];
        if !image_in_flight.is_null() {
            self.device
                .wait_for_fences(&[image_in_flight], true, u64::MAX)?;
        }

        // 5. record fence which use swapchain image
        self.data.images_in_flight[image_index] = in_flight_fence;

        // if image_index is updated from render function,
        // uniform_buffer[index] is updated
        // and then reflect shader via descriptor sets which is binding with pipeline
        // uniform buffer <-> descriptor set <-> pipeline layout <-> pipeline <-> shader
        update_command_buffer(self, image_index)?;
        update_uniform_buffer(self, image_index)?;
        update_light_uniform_buffer(self, image_index)?;

        // 6. wait image_available_semaphores
        let wait_semaphores = &[self.data.image_available_semaphores[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        // 7. copy using command buffer
        let command_buffers = &[self.data.command_buffers[image_index]];
        // Presentation can outlive the frame fence, so keep one signal
        // semaphore per swapchain image and only reuse it after that image is
        // acquired again.
        // 8. signal to render_finished_semaphore
        let signal_semaphores = &[self.data.render_finished_semaphores[image_index]];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        // 9. reset fence
        self.device.reset_fences(&[in_flight_fence])?;

        // 10. submit to graphics queue
        self.device
            .queue_submit(self.data.graphics_queue, &[submit_info], in_flight_fence)?;

        let swapchains = &[self.data.swapchain];
        let image_indices = &[image_index as u32];

        // 11. present_queue wait for render_finished_semaphore
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        // 12. presentation
        let result = self
            .device
            .queue_present_khr(self.data.present_queue, &present_info);
        // 13. if window is resized, recreate swapchain
        let changed = acquire_code == vk::SuccessCode::SUBOPTIMAL_KHR
            || result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);
        if self.resized || changed {
            self.resized = false;
            self.recreate_swapchain(window)?;
        } else if let Err(e) = result {
            return Err(anyhow!(e));
        }

        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    // load 3d mesh from model from designated path
    pub unsafe fn load_mesh3d_from_model(&mut self, path: &str) -> Result<MeshHandle> {
        let source = load_model_source(path)?;
        let mesh_data = source.to_mesh3d_data();

        self.load_mesh_from_data(mesh_data, VertexLayout::Mesh3D)
    }

    pub unsafe fn load_debug_line_from_model(&mut self, path: &str) -> Result<MeshHandle> {
        let source = load_model_source(path)?;
        let mesh_data = source.to_debugline_data();

        self.load_mesh_from_data(mesh_data, VertexLayout::DebugLine3D)
    }

    pub unsafe fn load_lit3d_from_model(&mut self, path: &str) -> Result<MeshHandle> {
        let source = load_model_source(path)?;
        let mesh_data = source.to_lit3d_data();

        self.load_mesh_from_data(mesh_data, VertexLayout::Lit3D)
    }

    pub unsafe fn load_texture(&mut self, path: &str) -> Result<TextureHandle> {
        let texture = create_texture(&self.instance, &self.device, &mut self.data, path)?;
        let index = self.data.textures.len();
        self.data.textures.push(texture);

        if !self.data.descriptor_pool.is_null() && !self.data.uniform_buffers.is_empty() {
            self.device.device_wait_idle()?;
            self.device
                .destroy_descriptor_pool(self.data.descriptor_pool, None);
            self.data.global_descriptor_sets.clear();
            self.data.material_descriptor_sets.clear();
            self.data.light_descriptor_sets.clear();
            self.data.skybox_descriptor_sets.clear();
            create_descriptor_pool(&self.device, &mut self.data)?;
            create_global_descriptor_sets(&self.device, &mut self.data)?;
            create_material_descriptor_sets(&self.device, &mut self.data)?;
            create_light_descriptor_sets(&self.device, &mut self.data)?;
            create_skybox_descriptor_sets(&self.device, &mut self.data)?;
        }

        Ok(TextureHandle(index))
    }

    pub unsafe fn load_skybox_texture(&mut self, paths: [&str; 6]) -> Result<SkyboxTextureHandle> {
        let texture = create_skybox_texture(&self.instance, &self.device, &mut self.data, paths)?;

        let index = self.data.skybox_textures.len();
        self.data.skybox_textures.push(texture);

        if !self.data.descriptor_pool.is_null() && !self.data.uniform_buffers.is_empty() {
            self.device
                .destroy_descriptor_pool(self.data.descriptor_pool, None);

            self.data.global_descriptor_sets.clear();
            self.data.material_descriptor_sets.clear();
            self.data.light_descriptor_sets.clear();
            self.data.skybox_descriptor_sets.clear();

            create_descriptor_pool(&self.device, &mut self.data)?;
            create_global_descriptor_sets(&self.device, &mut self.data)?;
            create_material_descriptor_sets(&self.device, &mut self.data)?;
            create_light_descriptor_sets(&self.device, &mut self.data)?;
            create_skybox_descriptor_sets(&self.device, &mut self.data)?;
        }

        Ok(SkyboxTextureHandle(index))
    }

    pub unsafe fn set_skybox(
        &mut self,
        mesh: MeshHandle,
        texture: SkyboxTextureHandle,
    ) -> Result<()> {
        if mesh.vertex_layout != VertexLayout::Skybox {
            return Err(anyhow!("Skybox mesh must use VertexLayout::Skybox."));
        }

        if self.data.skybox_textures.get(texture.0).is_none() {
            return Err(anyhow!("Skybox texture index out of range: {}", texture.0));
        }

        self.data.skybox = Some(RenderSkybox {
            mesh,
            texture,
            is_visible: true,
        });

        if !self.data.descriptor_pool.is_null() && !self.data.uniform_buffers.is_empty() {
            self.device.device_wait_idle()?;
            self.device
                .destroy_descriptor_pool(self.data.descriptor_pool, None);
            self.data.global_descriptor_sets.clear();
            self.data.material_descriptor_sets.clear();
            self.data.light_descriptor_sets.clear();
            self.data.skybox_descriptor_sets.clear();
            create_descriptor_pool(&self.device, &mut self.data)?;
            create_global_descriptor_sets(&self.device, &mut self.data)?;
            create_material_descriptor_sets(&self.device, &mut self.data)?;
            create_light_descriptor_sets(&self.device, &mut self.data)?;
            create_skybox_descriptor_sets(&self.device, &mut self.data)?;
        }

        Ok(())
    }

    pub fn clear_skybox(&mut self) {
        self.data.skybox = None;
    }

    // load mesh for simple polygon i.e. triangle, rectangle ..
    pub unsafe fn load_mesh_from_vertices<V>(
        &mut self,
        vertices: Vec<V>,
        indices: Vec<u32>,
        vertex_layout: VertexLayout,
    ) -> Result<MeshHandle> {
        self.load_mesh_from_data(MeshData { vertices, indices }, vertex_layout)
    }

    pub unsafe fn update_mesh_from_data<V>(
        &mut self,
        mesh_handle: MeshHandle,
        mesh_data: MeshData<V>,
        vertex_layout: VertexLayout,
    ) -> Result<()> {
        if mesh_data.vertices.is_empty() || mesh_data.indices.is_empty() {
            return Err(anyhow!("Mesh vertices and indices must not be empty."));
        }

        let mesh_index = mesh_handle.index;
        let mesh = self
            .data
            .meshes
            .get(mesh_index)
            .ok_or_else(|| anyhow!("Mesh index out of range: {mesh_index}"))?
            .as_ref()
            .ok_or_else(|| anyhow!("Mesh slot is empty: {mesh_index}"))?;

        let vertex_buffer_size = (size_of::<V>() * mesh_data.vertices.len()) as vk::DeviceSize;
        let index_buffer_size = (size_of::<u32>() * mesh_data.indices.len()) as vk::DeviceSize;

        let can_reuse_buffers = mesh.vertex_layout == vertex_layout
            && mesh.vertex_buffer_size == vertex_buffer_size
            && mesh.index_buffer_size == index_buffer_size;

        if can_reuse_buffers {
            self.upload_to_buffer(mesh.vertex_buffer, &mesh_data.vertices)?;
            self.upload_to_buffer(mesh.index_buffer, &mesh_data.indices)?;

            let mesh = self.data.meshes[mesh_index]
                .as_mut()
                .ok_or_else(|| anyhow!("Mesh slot is empty: {mesh_index}"))?;
            mesh.index_count = mesh_data.indices.len() as u32;
        } else {
            let new_mesh = create_mesh(
                &self.instance,
                &self.device,
                &self.data,
                mesh_data,
                vertex_layout,
            )?;

            self.device.device_wait_idle()?;

            // old mesh is released
            let old_mesh = self.data.meshes[mesh_index]
                .replace(new_mesh)
                .ok_or_else(|| anyhow!("Mesh slot is empty: {mesh_index}"))?;
            self.device.free_memory(old_mesh.index_buffer_memory, None);
            self.device.destroy_buffer(old_mesh.index_buffer, None);
            self.device.free_memory(old_mesh.vertex_buffer_memory, None);
            self.device.destroy_buffer(old_mesh.vertex_buffer, None);
        }

        Ok(())
    }

    unsafe fn upload_to_buffer<T>(&self, destination: vk::Buffer, values: &[T]) -> Result<()> {
        let size = (size_of::<T>() * values.len()) as vk::DeviceSize;
        let (staging_buffer, staging_buffer_memory) = create_buffer(
            &self.instance,
            &self.device,
            &self.data,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        let memory =
            self.device
                .map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;

        memcpy(values.as_ptr(), memory.cast(), values.len());
        self.device.unmap_memory(staging_buffer_memory);

        copy_buffer(&self.device, &self.data, staging_buffer, destination, size)?;

        self.device.destroy_buffer(staging_buffer, None);
        self.device.free_memory(staging_buffer_memory, None);

        Ok(())
    }

    pub unsafe fn load_mesh_from_data<V>(
        &mut self,
        mesh_data: MeshData<V>,
        vertex_layout: VertexLayout,
    ) -> Result<MeshHandle> {
        let mesh = create_mesh(
            &self.instance,
            &self.device,
            &self.data,
            mesh_data,
            vertex_layout,
        )?;
        self.data.meshes.push(Some(mesh));

        Ok(MeshHandle::new(self.data.meshes.len() - 1, vertex_layout))
    }

    pub unsafe fn destroy_mesh(&mut self, mesh_handle: MeshHandle) -> Result<()> {
        let mesh_index = mesh_handle.index;
        let slot = self
            .data
            .meshes
            .get_mut(mesh_index)
            .ok_or_else(|| anyhow!("Mesh index out of range: {mesh_index}"))?;

        let Some(mesh) = slot.take() else {
            return Ok(());
        };

        if mesh.vertex_layout != mesh_handle.vertex_layout {
            *slot = Some(mesh);
            return Err(anyhow!(
                "MeshHandle vertex layout {:?} does not match mesh slot {:?}.",
                mesh_handle.vertex_layout,
                slot.as_ref().unwrap().vertex_layout
            ));
        }

        self.device.device_wait_idle()?;
        self.device.free_memory(mesh.index_buffer_memory, None);
        self.device.destroy_buffer(mesh.index_buffer, None);
        self.device.free_memory(mesh.vertex_buffer_memory, None);
        self.device.destroy_buffer(mesh.vertex_buffer, None);

        Ok(())
    }

    pub fn set_render_items(&mut self, render_items: Vec<RenderItem>) {
        self.data.render_objects = render_items;
    }

    pub fn clear_render_items(&mut self) {
        self.data.render_objects.clear();
    }

    pub fn set_camera(&mut self, camera: RenderCamera) {
        self.data.camera = camera;
    }

    // font /////////////////////////////////////////////////////////////////////////////
    /* let data = std::fs::read(&path).context("failed to read font file")?;
    let font = system.load_font(data)?; */
    pub fn load_font(&mut self, bytes: Vec<u8>) -> Result<FontHandle>{
        let font=self.text_system.load_font(bytes)?;

        let handle=FontHandle(self.fonts.len());
        self.fonts.push(font);

        Ok(handle)
    }

    // send atlas map to gpu
    pub unsafe fn upload_text_atlas(&mut self) -> Result<()>{
        self.gpu_glyph_atlas.upload_dirty_pages(
            &self.instance,
            &self.device,
            &mut self.data,
            &mut self.glyph_atlas,
        )?;

        self.gpu_glyph_atlas.ensure_descriptors(
            &self.device, 
            &self.data
        )
    }

    // send meshes to gpu
    pub unsafe fn upload_text_mesh(
        &mut self,
        text_mesh: &kani_volcano_text::TextMesh,
    ) -> Result<GpuTextMesh> {
        use cgmath::{vec2, vec3};
        use crate::vertex::Ui2DVertex;

        let mut result = GpuTextMesh::default();

        for batch in &text_mesh.batches {
            if batch.vertices.is_empty() || batch.indices.is_empty() {
                continue;
            }

            let vertices = batch.vertices.iter()
                .map(|vertex| {
                    Ui2DVertex::new(
                        vec2(
                            vertex.position[0],
                            vertex.position[1],
                        ),
                        vec3(1.0, 1.0, 1.0),
                        vec2(vertex.uv[0], vertex.uv[1]),
                    )
                })
                .collect();

            let mesh_data = MeshData {
                vertices,
                indices: batch.indices.clone(),
            };

            let handle = match self.load_mesh_from_data(
                mesh_data,
                VertexLayout::Ui2D,
            ) {
                Ok(handle) => handle,
                Err(error) => {
                    for uploaded in &result.batches {
                        self.destroy_mesh(uploaded.mesh)?;
                    }
                    return Err(error);
                }
            };

            result.batches.push(GpuTextBatch {
                page: batch.page,
                mesh: handle,
            });
        }

        Ok(result)
    }

    pub unsafe fn destroy_text_mesh(&mut self, text_mesh: &mut GpuTextMesh) -> Result<()>{
        while let Some(batch) = text_mesh.batches.last(){
            self.destroy_mesh(batch.mesh)?;
            text_mesh.batches.pop();
        }

        Ok(())
    }

    unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        self.device.device_wait_idle()?;
        self.destroy_swapchain();
        create_swapchain(window, &self.instance, &self.device, &mut self.data)?;
        create_swapchain_image_views(&self.device, &mut self.data)?;
        create_render_pass(&self.instance, &self.device, &mut self.data)?;
        create_pipelines(&self.device, &mut self.data)?;
        create_color_objects(&self.instance, &self.device, &mut self.data)?;
        create_depth_objects(&self.instance, &self.device, &mut self.data)?;
        create_framebuffers(&self.device, &mut self.data)?;
        create_uniform_buffers(&self.instance, &self.device, &mut self.data)?;
        create_light_uniform_buffers(&self.instance, &self.device, &mut self.data)?;
        create_descriptor_pool(&self.device, &mut self.data)?;
        create_global_descriptor_sets(&self.device, &mut self.data)?;
        create_material_descriptor_sets(&self.device, &mut self.data)?;
        create_light_descriptor_sets(&self.device, &mut self.data)?;
        create_skybox_descriptor_sets(&self.device, &mut self.data)?;
        create_command_buffers(&self.device, &mut self.data)?;
        create_render_finished_semaphores(&self.device, &mut self.data)?;
        self.data.images_in_flight = vec![vk::Fence::null(); self.data.swapchain_images.len()];
        Ok(())
    }

    pub unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();

        self.destroy_swapchain();

        self.gpu_glyph_atlas.destroy(&self.device);

        self.data
            .command_pools
            .iter()
            .for_each(|p| self.device.destroy_command_pool(*p, None));

        self.device.destroy_sampler(self.data.texture_sampler, None);

        self.data.textures.drain(..).for_each(|texture| {
            self.device.destroy_image_view(texture.image_view, None);
            self.device.destroy_image(texture.image, None);
            self.device.free_memory(texture.image_memory, None);
        });
        self.data.skybox_textures.drain(..).for_each(|texture| {
            self.device.destroy_image_view(texture.image_view, None);
            self.device.destroy_image(texture.image, None);
            self.device.free_memory(texture.image_memory, None);
        });

        self.device
            .destroy_descriptor_set_layout(self.data.material_descriptor_set_layout, None);
        self.device
            .destroy_descriptor_set_layout(self.data.light_descriptor_set_layout, None);
        self.device
            .destroy_descriptor_set_layout(self.data.global_descriptor_set_layout, None);
        self.device
            .destroy_descriptor_set_layout(self.data.skybox_descriptor_set_layout, None);

        self.data
            .in_flight_fences
            .iter()
            .for_each(|f| self.device.destroy_fence(*f, None));
        self.data
            .render_finished_semaphores
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));
        self.data
            .image_available_semaphores
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));
        self.data.meshes.drain(..).flatten().for_each(|mesh| {
            self.device.free_memory(mesh.index_buffer_memory, None);
            self.device.destroy_buffer(mesh.index_buffer, None);
            self.device.free_memory(mesh.vertex_buffer_memory, None);
            self.device.destroy_buffer(mesh.vertex_buffer, None);
        });
        self.device
            .destroy_command_pool(self.data.command_pool, None);
        self.device.destroy_device(None);
        self.instance.destroy_surface_khr(self.data.surface, None);

        if VALIDATION_ENABLED {
            self.instance
                .destroy_debug_utils_messenger_ext(self.data.messenger, None);
        }

        self.instance.destroy_instance(None);
    }

    unsafe fn destroy_swapchain(&mut self) {
        self.data.command_buffers.clear();
        self.device
            .destroy_descriptor_pool(self.data.descriptor_pool, None);
        self.data
            .uniform_buffers
            .drain(..)
            .for_each(|b| self.device.destroy_buffer(b, None));
        self.data
            .uniform_buffers_memory
            .drain(..)
            .for_each(|m| self.device.free_memory(m, None));
        self.data.global_descriptor_sets.clear();
        self.data.material_descriptor_sets.clear();
        self.data.light_descriptor_sets.clear();
        self.data.skybox_descriptor_sets.clear();
        self.data
            .light_uniform_buffers
            .drain(..)
            .for_each(|b| self.device.destroy_buffer(b, None));
        self.data
            .light_uniform_buffers_memory
            .drain(..)
            .for_each(|m| self.device.free_memory(m, None));
        self.data
            .render_finished_semaphores
            .drain(..)
            .for_each(|s| self.device.destroy_semaphore(s, None));
        self.data
            .framebuffers
            .drain(..)
            .for_each(|f| self.device.destroy_framebuffer(f, None));
        self.device
            .destroy_image_view(self.data.color_image_view, None);
        self.device.destroy_image(self.data.color_image, None);
        self.device.free_memory(self.data.color_image_memory, None);
        self.device
            .destroy_image_view(self.data.depth_image_view, None);
        self.device.destroy_image(self.data.depth_image, None);
        self.device.free_memory(self.data.depth_image_memory, None);
        self.data.pipelines.drain(..).for_each(|p| {
            self.device.destroy_pipeline(p.pipeline, None);
            self.device.destroy_pipeline_layout(p.layout, None);
        });
        self.device.destroy_render_pass(self.data.render_pass, None);
        self.data
            .swapchain_image_views
            .iter()
            .for_each(|v| self.device.destroy_image_view(*v, None));
        self.device.destroy_swapchain_khr(self.data.swapchain, None);
    }
}

unsafe fn create_pipelines(device: &Device, data: &mut VulkanData) -> Result<()> {
    create_mesh3d_pipeline(device, data)?;
    create_debug_line_pipeline(device, data)?;
    create_transparent3d_pipeline(device, data)?;
    create_lit3d_pipeline(device, data)?;
    create_ui2d_pipeline(device, data)?;
    create_text_ui2d_pipeline(device, data)?;
    create_skybox_pipeline(device, data)?;

    Ok(())
}

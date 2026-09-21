use anyhow::{Result, anyhow};
use cgmath::vec3;
use kani_volcano_math::Transform;
use renderer_vulkan::{
    RenderCamera, RenderItem, SkyboxTextureHandle, VulkanRenderer,
    TextRenderItem,
};

use super::render_command::{RenderCommand, RenderCommandQueue};
use crate::AssetApi;
use crate::primitive::{PrimitiveMesh, create_primitive_with_layout, update_primitive_mesh};
use crate::{EntityId, MeshAssetId, Resources, World};

use crate::component::{Camera, Component, MeshRenderer, PendingPrimitiveMesh, Visibility};

pub struct RenderContext<'a> {
    world: &'a mut World,
    resources: &'a mut Resources,
    renderer: &'a mut VulkanRenderer,
    render_commands: &'a mut RenderCommandQueue,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        world: &'a mut World,
        resources: &'a mut Resources,
        renderer: &'a mut VulkanRenderer,
        render_commands: &'a mut RenderCommandQueue,
    ) -> Self {
        Self {
            world,
            resources,
            renderer,
            render_commands,
        }
    }

    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) -> bool {
        self.world.add_component::<T>(entity, component)
    }

    pub fn remove_component<T: Component>(&mut self, entity: EntityId) -> Option<T> {
        self.world.remove_component::<T>(entity)
    }

    pub fn query1<A>(&self) -> Box<dyn Iterator<Item=(EntityId,&A)> + '_>
    where 
        A: Component,
    {
        self.world.query1::<A>()
    }

    pub fn query2<A, B>(&self) -> Box<dyn Iterator<Item = (EntityId, &A, &B)> + '_>
    where
        A: Component,
        B: Component,
    {
        self.world.query2::<A, B>()
    }

    pub fn get_component<T: Component>(&self, entity: EntityId) -> Option<&T> {
        self.world.get_component::<T>(entity)
    }

    pub(crate) fn set_render_items(&mut self, render_items: Vec<RenderItem>) {
        self.renderer.set_render_items(render_items);
    }

    pub(crate) fn set_text_render_items(&mut self, text_render_items: Vec<TextRenderItem>){
        self.renderer.set_text_render_items(text_render_items);
    }

    pub(crate) fn set_camera(&mut self, camera: RenderCamera) {
        self.renderer.set_camera(camera);
    }

    pub(crate) unsafe fn create_text_mesh(
        &mut self,
        font: crate::FontAssetId,
        content: &str,
        font_size: f32,
        line_height: f32,
    ) -> Result<renderer_vulkan::GpuTextMesh> {
        let handle = self.resources.font_asset(font)
            .ok_or_else(|| anyhow!("font asset not found: {:?}", font))?.handle;
        self.renderer.create_text_mesh(handle, content, font_size, line_height)
    }

    pub(crate) unsafe fn destroy_text_mesh(
        &mut self,
        mesh: &mut renderer_vulkan::GpuTextMesh,
    ) -> Result<()> {
        self.renderer.destroy_text_mesh(mesh)
    }

    pub(crate) unsafe fn destroy_mesh(&mut self, mesh: renderer_vulkan::MeshHandle) -> Result<()> {
        self.renderer.destroy_mesh(mesh)
    }

    pub(crate) unsafe fn set_skybox(
        &mut self,
        mesh: renderer_vulkan::MeshHandle,
        texture: SkyboxTextureHandle,
    ) -> Result<()> {
        self.renderer.set_skybox(mesh, texture)
    }

    pub(crate) unsafe fn update_primitive_mesh(
        &mut self,
        mesh: PrimitiveMesh,
        shape: crate::PrimitiveShape,
    ) -> Result<()> {
        update_primitive_mesh(self.renderer, self.resources, mesh, shape)
    }

    pub(crate) unsafe fn create_primitive_mesh(
        &mut self,
        pending: PendingPrimitiveMesh,
    ) -> Result<PrimitiveMesh> {
        let vertex_layout = pending.material.pipeline_key.required_vertex_layout();

        create_primitive_with_layout(
            self.renderer,
            self.resources,
            pending.shape,
            vertex_layout,
            pending.auto_release,
        )
    }

    pub(crate) fn register_primitive_mesh(&mut self, mesh: PrimitiveMesh) -> PrimitiveMesh {
        self.resources.register_primitive_mesh(mesh)
    }

    pub(crate) fn retain_mesh(
        &mut self,
        asset_id: MeshAssetId,
    ) -> Option<renderer_vulkan::MeshHandle> {
        self.resources.retain_mesh(asset_id)
    }

    pub(crate) fn release_mesh(
        &mut self,
        asset_id: MeshAssetId,
    ) -> Option<renderer_vulkan::MeshHandle> {
        self.resources.release_mesh(asset_id)
    }

    pub(crate) fn drain_render_commands(&mut self) -> Vec<RenderCommand> {
        self.render_commands.drain().collect()
    }
}

impl AssetApi for RenderContext<'_> {
    fn resources(&self) -> &Resources {
        &self.resources
    }

    fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }
}

#[derive(Clone, Debug)]
pub struct RenderSystem;

impl RenderSystem {
    pub fn update(&mut self, context: &mut RenderContext<'_>) -> Result<()> {
        self.execute_render_commands(context)?;

        // render entities whitch have MeshRenderer and Transform
        // if entity have Visibility and it is false, this entity is not be rendered
        let render_items = context
            .query2::<Transform, MeshRenderer>()
            .map(|(entity, transform, mesh_renderer)| RenderItem {
                mesh_index: mesh_renderer.mesh,
                transform: transform.clone(),
                alpha: mesh_renderer.material.alpha,
                material_color: mesh_renderer.material.color,
                use_texture: mesh_renderer.material.use_texture,
                texture_index: mesh_renderer.material.texture,
                pipeline_key: mesh_renderer.material.pipeline_key,
                is_visible: context
                    .get_component::<Visibility>(entity)
                    .is_none_or(|visibility| visibility.is_visible),
            })
            .collect::<Vec<_>>();

        context.set_render_items(render_items);

        let render_camera =
            context
                .query2::<Transform, Camera>()
                .next()
                .map(|(_, transform, camera)| RenderCamera {
                    position: transform.position,
                    target: camera.target,
                    up: vec3(0.0, 0.0, 1.0),
                    fov_y: camera.fov_y,
                    near: camera.near,
                    far: camera.far,
                });

        if let Some(render_camera) = render_camera {
            context.set_camera(render_camera);
        }

        Ok(())
    }

    pub fn execute_render_commands(&mut self, context: &mut RenderContext<'_>) -> Result<()> {
        let commands = context.drain_render_commands();

        for command in commands {
            match command {
                RenderCommand::DestroyMesh { mesh } => unsafe {
                    context.destroy_mesh(mesh)?;
                },
                RenderCommand::UpdatePrimitiveMesh { asset_id, shape } => {
                    let primitive_type = context
                        .primitive_type_from_asset_id(asset_id)
                        .ok_or_else(|| anyhow!("not found primitive_type from: {asset_id:?}"))?;

                    let vertex_layout = context
                        .vertex_layout_from_asset_id(asset_id)
                        .ok_or_else(|| anyhow!("not found primitive_type from: {asset_id:?}"))?;

                    if let Some(asset_id) =
                        context.primitive_asset_id(primitive_type, vertex_layout)
                    {
                        let primitive_mesh = PrimitiveMesh {
                            asset_id,
                            primitive_type,
                            vertex_layout,
                        };
                        unsafe {
                            context.update_primitive_mesh(primitive_mesh, shape)?;
                        }
                    }
                }
                RenderCommand::CreatePrimitiveMesh { entity } => {
                    let Some(pending) = context
                        .get_component::<PendingPrimitiveMesh>(entity)
                        .cloned()
                    else {
                        continue;
                    };

                    // new mesh is created here (MeshAssetId)
                    let primitive_mesh = unsafe { context.create_primitive_mesh(pending.clone())? };

                    context.register_primitive_mesh(primitive_mesh);

                    let mesh = context
                        .retain_mesh(primitive_mesh.asset_id)
                        .ok_or_else(|| {
                            anyhow!("mesh asset not found: {:?}", primitive_mesh.asset_id)
                        })?;

                    let mesh_renderer = match MeshRenderer::new(mesh, pending.material) {
                        Ok(mesh_renderer) => mesh_renderer.with_asset_id(primitive_mesh.asset_id),
                        Err(error) => {
                            context.release_mesh(primitive_mesh.asset_id);
                            return Err(error);
                        }
                    };

                    context.add_component(entity, mesh_renderer);
                    context.remove_component::<PendingPrimitiveMesh>(entity);
                }
                RenderCommand::SetSkybox { mesh, texture } => unsafe {
                    context.set_skybox(mesh, texture)?;
                },
            }
        }

        Ok(())
    }
}

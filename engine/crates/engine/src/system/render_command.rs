use renderer_vulkan::{MeshHandle, SkyboxTextureHandle, Msaa};

use crate::{EntityId, MeshAssetId, PrimitiveShape};

#[derive(Debug)]
pub enum RenderCommand {
    DestroyMesh {
        mesh: MeshHandle,
    },
    UpdatePrimitiveMesh {
        asset_id: MeshAssetId,
        shape: PrimitiveShape,
    },
    CreatePrimitiveMesh {
        entity: EntityId,
    },
    SetSkybox {
        mesh: MeshHandle,
        texture: SkyboxTextureHandle,
    },
    SetMsaa{
        samples: Msaa
    },
}

#[derive(Debug)]
pub struct RenderCommandQueue {
    commands: Vec<RenderCommand>,
}

impl RenderCommandQueue {
    pub fn destroy_mesh(&mut self, mesh: MeshHandle) {
        self.commands.push(RenderCommand::DestroyMesh { mesh });
    }

    pub fn update_primitive_mesh(&mut self, asset_id: MeshAssetId, shape: PrimitiveShape) {
        self.commands
            .push(RenderCommand::UpdatePrimitiveMesh { asset_id, shape });
    }

    pub fn create_primitive_mesh(&mut self, entity: EntityId) {
        self.commands
            .push(RenderCommand::CreatePrimitiveMesh { entity });
    }

    pub fn set_skybox(&mut self, mesh: MeshHandle, texture: SkyboxTextureHandle) {
        self.commands
            .push(RenderCommand::SetSkybox { mesh, texture });
    }

    pub fn set_msaa(&mut self, msaa: Msaa){
        self.commands
            .push(RenderCommand::SetMsaa{ samples: msaa });
    }
    pub fn drain(&mut self) -> impl Iterator<Item = RenderCommand> + '_ {
        self.commands.drain(..)
    }

}

impl Default for RenderCommandQueue {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

use crate::{MeshAssetId, Msaa, PrimitiveShape, RenderCommandQueue};

pub trait RenderCommandApi {
    fn render_commands_mut(&mut self) -> &mut RenderCommandQueue;

    /// Queues an MSAA change for the next render stage.
    /// Unsupported settings are reported when the command is executed.
    fn set_msaa(&mut self, msaa: Msaa) {
        self.render_commands_mut().set_msaa(msaa);
    }

    fn update_primitive_mesh(&mut self, asset_id: MeshAssetId, shape: PrimitiveShape) {
        self.render_commands_mut()
            .update_primitive_mesh(asset_id, shape);
    }
}

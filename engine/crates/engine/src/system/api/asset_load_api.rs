use renderer_vulkan::{PipelineKey};
use crate::{AssetLoadCommandQueue};

pub trait AssetLoadApi{
    fn asset_load_commands_mut(&mut self) -> &mut AssetLoadCommandQueue;

    fn request_load_model(
        &mut self,
        name: &str,
        path: &str,
        pipeline_key: PipelineKey,
        auto_release: bool,
    ){
        self.asset_load_commands_mut()
            .request_load_model(name,path,pipeline_key,auto_release);
    }

    fn request_load_texture(
        &mut self,
        name: &str,
        path: &str,
    ){
        self.asset_load_commands_mut()
            .request_load_texture(name,path);
    }

    fn request_load_skybox_texture(
        &mut self,
        name: &str,
        paths: [&str;6]
    ){
        self.asset_load_commands_mut()
            .request_load_skybox_texture(name, paths);
    }

    fn request_load_font(
        &mut self,
        name: &str,
        path: &str
    ){
        self.asset_load_commands_mut()
            .request_load_font(name,path);
    }

    fn request_load_audio(
        &mut self,
        name: &str,
        path: &str
    ){
        self.asset_load_commands_mut()
            .request_load_audio(name, path);
    }


}
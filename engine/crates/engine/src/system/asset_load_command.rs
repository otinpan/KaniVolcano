use renderer_vulkan::{PipelineKey};
#[derive(Debug)]
pub enum AssetLoadCommand{
    LoadModel{
        name: String,
        path: String,
        pipeline_key: PipelineKey,
        auto_release: bool,
    },
    LoadTexture{
        name: String,
        path: String,
    },
    LoadSkyboxTexture{
        name: String,
        paths: [String;6],
    },
    LoadFont{
        name: String,
        path: String,
    },
    LoadAudio{
        name: String,
        path: String,
    },
}


#[derive(Default)]
pub struct AssetLoadCommandQueue{
    commands: Vec<AssetLoadCommand>,
}

impl AssetLoadCommandQueue{
    pub(crate) fn drain(&mut self) -> impl Iterator<Item=AssetLoadCommand> + '_{
        self.commands.drain(..)
    }

    pub fn is_empty(&self) -> bool{
        self.commands.is_empty()
    }

    pub fn request_load_model(
        &mut self,
        name: &str,
        path: &str,
        pipeline_key: PipelineKey,
        auto_release: bool,
    ){
        self.commands.push(AssetLoadCommand::LoadModel {
            name: name.to_string(),
            path: path.to_string(),
            pipeline_key,
            auto_release,
        });
    }
    
    pub fn request_load_texture(
        &mut self,
        name: &str,
        path: &str,
    ){
        self.commands.push(AssetLoadCommand::LoadTexture{
            name: name.to_string(),
            path: path.to_string(),
        });
    }

    pub fn request_load_skybox_texture(
        &mut self,
        name: &str,
        paths: [&str;6]
    ){
        self.commands.push(AssetLoadCommand::LoadSkyboxTexture { 
            name: name.to_string(), 
            paths: paths.map(String::from),
        });
    }

    pub fn request_load_font(
        &mut self,
        name: &str,
        path: &str,
    ){
        self.commands.push(AssetLoadCommand::LoadFont {
            name: name.to_string(), 
            path: path.to_string()
        });
    }

    pub fn request_load_audio(
        &mut self,
        name: &str,
        path: &str,
    ){
        self.commands.push(AssetLoadCommand::LoadAudio{
            name: name.to_string(),
            path: path.to_string(),
        });
    }
}

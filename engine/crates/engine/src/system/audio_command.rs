use kani_volcano_audio::{
    ReverbSettings, PlaybackSettings,
};
use crate::resources::{
    AudioBusId, AudioAssetId, AudioEmitterId, AudioPlaybackId,
};


// --Bus----------------
#[derive(Debug)]
pub struct AudioBusSettings{
    pub volume: f32,
    pub bus_type: AudioBusType,
}

#[derive(Debug)]
pub enum AudioBusType{
    Sub{
        output: AudioBusId,
    },
    Reverb{
        settings: ReverbSettings,
    }
}

// --Emitter------------------------
#[derive(Debug)]
pub struct AudioEmitterSettings{
    pub output: AudioBusId,
    pub volume: f32, 
    pub panning: f32,
    pub sends: Vec<AudioSend>,
}
#[derive(Debug)]
pub struct AudioSend{
    pub bus: AudioBusId,
    pub gain: f32,
}


pub enum AudioCommand{
    CreateBus{
        name: String,
        settings: AudioBusSettings,
    },
    CreateEmitter{
        name: String,
        settings: AudioEmitterSettings,
    },
    // play on master
    Play{
        playback: AudioPlaybackId,
        audio: AudioAssetId,
        settings: PlaybackSettings,
    },
    PlayOnBus{
        playback: AudioPlaybackId,
        audio: AudioAssetId,
        bus: AudioBusId,
        settings: PlaybackSettings,
    },
    PlayOnEmitter{
        playback: AudioPlaybackId,
        audio: AudioAssetId, 
        emitter: AudioEmitterId,
        settings: PlaybackSettings,
    },
    Stop{
        playback: AudioPlaybackId,
        fade_seconds: f32,
    },
    Pause{
        playback: AudioPlaybackId,
        fade_seconds: f32,
    },
    Resume{
        playback: AudioPlaybackId,
        fade_seconds: f32,
    },
    SetPlaybackVolume{
        playback: AudioPlaybackId,
        volume: f32,
        fade_seconds: f32,
    },
    SetBusVolume{
        bus: AudioBusId,
        volume: f32,
        fade_seconds: f32,
    },
    SetBusReverb{
        bus: AudioBusId,
        settings: ReverbSettings,
        fade_seconds: f32,
    },
    SetEmitterVolume{
        emitter: AudioEmitterId,
        volume: f32,
        fade_seconds: f32,
    },
    SetEmitterPanning{
        emitter: AudioEmitterId,
        panning: f32,
        fade_seconds: f32,
    },
}

#[derive(Default)]
pub struct AudioCommandQueue{
    commands: Vec<AudioCommand>,
}

impl AudioCommandQueue{
    pub(crate) fn drain(&mut self) -> impl Iterator<Item=AudioCommand> + '_{
        self.commands.drain(..)
    }

    pub fn is_empty(&self) -> bool{
        self.commands.is_empty()
    }

    pub fn create_bus(
        &mut self,
        name: &str,
        settings: AudioBusSettings,
    ){
        self.commands.push(AudioCommand::CreateBus { 
            name: name.to_string(),
            settings
        });
    }
    pub fn create_emitter(
        &mut self,
        name: &str,
        settings: AudioEmitterSettings,
    ){
        self.commands.push(AudioCommand::CreateEmitter { 
            name: name.to_string(),
            settings 
        })
    }
    pub fn play(
        &mut self,
        playback: AudioPlaybackId,
        audio: AudioAssetId,
        settings: PlaybackSettings,
    ){
        self.commands.push(AudioCommand::Play {playback, audio, settings })
    }
    pub fn play_on_bus(
        &mut self,
        playback: AudioPlaybackId,
        audio: AudioAssetId,
        bus: AudioBusId,
        settings: PlaybackSettings,
    ){
        self.commands.push(AudioCommand::PlayOnBus { playback, audio, bus, settings })
    }
    pub fn play_on_emitter(
        &mut self,
        playback: AudioPlaybackId,
        audio: AudioAssetId,
        emitter: AudioEmitterId,
        settings: PlaybackSettings,
    ){
        self.commands.push(AudioCommand::PlayOnEmitter {playback, audio, emitter, settings })
    }
    pub fn stop(
        &mut self,
        playback: AudioPlaybackId,
        fade_seconds: f32,
    ){
        self.commands.push(AudioCommand::Stop { playback, fade_seconds })
    }
    pub fn pause(
        &mut self,
        playback: AudioPlaybackId,
        fade_seconds: f32,
    ){
        self.commands.push(AudioCommand::Pause { playback, fade_seconds })
    }
    pub fn resume(
        &mut self,
        playback: AudioPlaybackId,
        fade_seconds: f32,
    ){
        self.commands.push(AudioCommand::Resume { playback, fade_seconds })
    }
    pub fn set_playback_volume(
        &mut self,
        playback: AudioPlaybackId,
        volume: f32,
        fade_seconds: f32,
    ) {
        self.commands.push(AudioCommand::SetPlaybackVolume {
            playback,
            volume,
            fade_seconds,
        });
    }

    pub fn set_bus_volume(
        &mut self,
        bus: AudioBusId,
        volume: f32,
        fade_seconds: f32,
    ) {
        self.commands.push(AudioCommand::SetBusVolume {
            bus,
            volume,
            fade_seconds,
        });
    }

    pub fn set_bus_reverb(
        &mut self,
        bus: AudioBusId,
        settings: ReverbSettings,
        fade_seconds: f32,
    ) {
        self.commands.push(AudioCommand::SetBusReverb {
            bus,
            settings,
            fade_seconds,
        });
    }

    pub fn set_emitter_volume(
        &mut self,
        emitter: AudioEmitterId,
        volume: f32,
        fade_seconds: f32,
    ) {
        self.commands.push(AudioCommand::SetEmitterVolume {
            emitter,
            volume,
            fade_seconds,
        });
    }

    pub fn set_emitter_panning(
        &mut self,
        emitter: AudioEmitterId,
        panning: f32,
        fade_seconds: f32
    ) {
        self.commands.push(AudioCommand::SetEmitterPanning{
            emitter,
            panning,
            fade_seconds,
        })
    }

}

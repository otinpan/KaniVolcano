use kira::{
    sound::static_sound::{
        StaticSoundHandle,
    }
};
use crate::bus::{AudioBusHandle,};
use crate::emitter::{AudioEmitterHandle,};
use crate::audio::{AudioHandle,};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaybackHandle(pub usize);

pub enum PlaybackTarget{
    Bus(AudioBusHandle),
    Emitter(AudioEmitterHandle),
}
pub struct PlaybackEntry{
    pub audio: AudioHandle,
    pub target: PlaybackTarget,
    pub sound: StaticSoundHandle,
}

pub struct PlaybackSettings{
    pub volume: f32,
    pub looping: bool,
}

impl Default for PlaybackSettings{
    fn default() -> Self{
        Self { volume: 1.0, looping: false, }
    }
}

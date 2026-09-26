use kira::{
    track::{TrackHandle},
};
use crate::{AudioBusHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AudioEmitterHandle(pub usize);


pub struct EmitterState{
    pub track: TrackHandle,
    pub panning: f32,
}
pub struct EmitterSettings{
    pub output: AudioBusHandle,
    pub volume: f32,
    pub panning: f32, // -1.0: left, 0.0: center, 1.0: right
    pub sends: Vec<AudioSend>,
}

pub struct AudioSend{
    pub bus: AudioBusHandle,
    pub gain: f32, // multiplier when send to bus 
}
use crate::{AudioBusHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AudioEmitterHandle(pub usize);

pub struct EmitterSettings{
    pub output: AudioBusHandle,
    pub volume: f32,
    pub sends: Vec<AudioSend>,
}

pub struct AudioSend{
    pub bus: AudioBusHandle,
    pub gain: f32,
}
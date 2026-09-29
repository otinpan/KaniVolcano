use super::Component;
use anyhow::{Result};
use crate::{AudioAssetId, AudioEmitterId};
use kani_volcano_audio::{PlaybackSettings};

pub struct AudioSource{
    pub audio: AudioAssetId,
    pub emitter: Option<AudioEmitterId>,
    pub settings: PlaybackSettings,
}

impl AudioSource{
    pub fn new(
        audio: AudioAssetId,
        emitter: Option<AudioEmitterId>,
        settings: PlaybackSettings,
    ) ->Result<Self>{
        Ok(Self { audio, emitter, settings })
    }

}

impl Component for AudioSource {}
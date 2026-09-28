use anyhow::{Result};
use crate::{
    AssetApi,AudioAssetId, AudioBusId, AudioCommandQueue, AudioEmitterId, AudioPlaybackId
};
use crate::system::audio_command::{
    AudioBusSettings, AudioEmitterSettings
};
use kani_volcano_audio::{PlaybackSettings, ReverbSettings};

/// Queues audio operations for later application by the engine.
pub trait AudioApi: AssetApi {
    fn master_bus(&self) -> Result<AudioBusId> {
        self.audio_bus_id("master")
    }

    fn audio_commands_mut(&mut self) -> &mut AudioCommandQueue;

    fn set_bus_panning(&mut self, bus: AudioBusId, panning: f32, fade_seconds: f32) {
        self.audio_commands_mut().set_bus_panning(bus, panning, fade_seconds);
    }

    fn destroy_emitter(&mut self, emitter: AudioEmitterId) {
        self.audio_commands_mut().destroy_emitter(emitter);
    }

    fn destroy_bus(&mut self, bus: AudioBusId) {
        self.audio_commands_mut().destroy_bus(bus);
    }

    fn create_bus(&mut self, name: &str, settings: AudioBusSettings) {
        self.audio_commands_mut().create_bus(name, settings);
    }

    fn create_emitter(&mut self, name: &str, settings: AudioEmitterSettings) {
        self.audio_commands_mut().create_emitter(name, settings);
    }

    fn play(&mut self, audio: AudioAssetId, settings: PlaybackSettings)-> Result<AudioPlaybackId> {
        let playback=self.resources_mut().reserve_audio_playback()?;

        self.audio_commands_mut().play(playback, audio, settings);
        Ok(playback)
    }

    fn play_on_bus(
        &mut self, 
        audio: AudioAssetId, 
        bus: AudioBusId, 
        settings: PlaybackSettings
    )-> Result<AudioPlaybackId>{
        let playback=self.resources_mut().reserve_audio_playback()?;

        self.audio_commands_mut().play_on_bus(playback, audio, bus, settings);
        Ok(playback)
    }

    fn play_on_emitter(
        &mut self,
        audio: AudioAssetId,
        emitter: AudioEmitterId,
        settings: PlaybackSettings,
    )-> Result<AudioPlaybackId> {
        let playback=self.resources_mut().reserve_audio_playback()?;

        self.audio_commands_mut().play_on_emitter(playback, audio, emitter, settings);
        Ok(playback)
    }

    fn stop(&mut self, playback: AudioPlaybackId, fade_seconds: f32) {
        self.audio_commands_mut().stop(playback, fade_seconds);
    }

    fn pause(&mut self, playback: AudioPlaybackId, fade_seconds: f32) {
        self.audio_commands_mut().pause(playback, fade_seconds);
    }

    fn resume(&mut self, playback: AudioPlaybackId, fade_seconds: f32) {
        self.audio_commands_mut().resume(playback, fade_seconds);
    }

    fn set_playback_volume(&mut self, playback: AudioPlaybackId, volume: f32, fade_seconds: f32) {
        self.audio_commands_mut().set_playback_volume(playback, volume, fade_seconds);
    }

    fn set_bus_volume(&mut self, bus: AudioBusId, volume: f32, fade_seconds: f32) {
        self.audio_commands_mut().set_bus_volume(bus, volume, fade_seconds);
    }

    fn set_bus_reverb(&mut self, bus: AudioBusId, settings: ReverbSettings, fade_seconds: f32) {
        self.audio_commands_mut().set_bus_reverb(bus, settings, fade_seconds);
    }

    fn set_emitter_volume(&mut self, emitter: AudioEmitterId, volume: f32, fade_seconds: f32) {
        self.audio_commands_mut().set_emitter_volume(emitter, volume, fade_seconds);
    }

    fn set_emitter_panning(&mut self, emitter: AudioEmitterId, panning: f32, fade_seconds: f32) {
        self.audio_commands_mut().set_emitter_panning(emitter, panning, fade_seconds);
    }
}

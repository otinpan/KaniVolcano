use anyhow::{Result,};
use crate::{
    AssetApi, AudioAssetId, AudioBusId, AudioCommandQueue, AudioEmitterId, AudioPlaybackId
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


    fn destroy_emitter(&mut self, emitter: &str) -> Result<()> {
        let id=self.audio_emitter_id(emitter)?;
        self.audio_commands_mut().destroy_emitter(id);

        Ok(())
    }
    fn destroy_emitter_from_id(& mut self, emitter: AudioEmitterId){
        self.audio_commands_mut().destroy_emitter(emitter);
    }

    fn destroy_bus(&mut self, bus: &str) -> Result<()> {
        let id=self.audio_bus_id(bus)?;
        self.audio_commands_mut().destroy_bus(id);
        Ok(())
    }
    fn destroy_bus_from_id(&mut self, bus: AudioBusId){
        self.audio_commands_mut().destroy_bus(bus);
    }

    fn create_bus(&mut self, name: &str, settings: AudioBusSettings) {
        self.audio_commands_mut().create_bus(name, settings);
    }

    fn create_emitter(&mut self, name: &str, settings: AudioEmitterSettings) {
        self.audio_commands_mut().create_emitter(name, settings);
    }

    fn play(&mut self, audio: &str, settings: PlaybackSettings)-> Result<AudioPlaybackId> {
        let id=self.audio_asset_id(audio)?;
        let playback=self.resources_mut().reserve_audio_playback()?;

        self.audio_commands_mut().play(playback, id, settings);
        Ok(playback)
    }
    fn play_from_id(&mut self, audio: AudioAssetId, settings: PlaybackSettings) -> Result<AudioPlaybackId>{
        let playback=self.resources_mut().reserve_audio_playback()?;
        self.audio_commands_mut().play(playback, audio, settings);
        Ok(playback)
    }

    fn play_on_bus(
        &mut self, 
        audio: &str, 
        bus: &str, 
        settings: PlaybackSettings
    )-> Result<AudioPlaybackId>{
        let audio_id=self.audio_asset_id(audio)?;
        let bus_id=self.audio_bus_id(bus)?;
        let playback=self.resources_mut().reserve_audio_playback()?;

        self.audio_commands_mut().play_on_bus(playback, audio_id, bus_id, settings);
        Ok(playback)
    }
    fn play_on_bus_from_id(
        &mut self,
        audio: AudioAssetId,
        bus: AudioBusId,
        settings: PlaybackSettings
    ) -> Result<AudioPlaybackId>{
        let playback=self.resources_mut().reserve_audio_playback()?;

        self.audio_commands_mut().play_on_bus(playback, audio, bus, settings);
        Ok(playback)
    }

    fn play_on_emitter(
        &mut self,
        audio: &str,
        emitter: &str,
        settings: PlaybackSettings,
    )-> Result<AudioPlaybackId> {
        let audio_id=self.audio_asset_id(audio)?;
        let emitter_id=self.audio_emitter_id(emitter)?;
        let playback=self.resources_mut().reserve_audio_playback()?;

        self.audio_commands_mut().play_on_emitter(playback, audio_id, emitter_id, settings);
        Ok(playback)
    }
    fn play_on_emitter_from_id(
        &mut self,
        audio: AudioAssetId,
        emitter: AudioEmitterId,
        settings: PlaybackSettings
    ) -> Result<AudioPlaybackId>{
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

    fn set_bus_volume(&mut self, bus: &str, volume: f32, fade_seconds: f32) -> Result<()> {
        let id=self.audio_bus_id(bus)?;
        self.audio_commands_mut().set_bus_volume(id, volume, fade_seconds);

        Ok(())
    }
    fn set_bus_volume_from_id(&mut self, bus: AudioBusId, volume: f32, fade_seconds: f32){
        self.audio_commands_mut().set_bus_volume(bus, volume, fade_seconds);
    }

    fn set_bus_panning(&mut self, bus: &str, panning: f32, fade_seconds: f32) -> Result<()>{
        let id=self.audio_bus_id(bus)?;

        self.audio_commands_mut().set_bus_panning(id, panning, fade_seconds);

        Ok(())
    }
    fn set_bus_panning_from_id(&mut self, bus: AudioBusId, panning: f32, fade_seconds: f32){
        self.audio_commands_mut().set_bus_panning(bus,panning,fade_seconds);
    }

    fn set_bus_reverb(&mut self, bus: &str, settings: ReverbSettings, fade_seconds: f32) -> Result<()> {
        let id=self.audio_bus_id(bus)?;
        self.audio_commands_mut().set_bus_reverb(id, settings, fade_seconds);

        Ok(())
    }
    fn set_bus_reverb_from_id(&mut self, bus: AudioBusId, settings: ReverbSettings, fade_seconds: f32) {
        self.audio_commands_mut().set_bus_reverb(bus, settings, fade_seconds);
    }

    fn set_emitter_volume(&mut self, emitter: &str, volume: f32, fade_seconds: f32) -> Result<()>{
        let id=self.audio_emitter_id(emitter)?;
        self.audio_commands_mut().set_emitter_volume(id, volume, fade_seconds);

        Ok(())
    }
    fn set_emitter_volume_from_id(&mut self, emitter: AudioEmitterId, volume: f32, fade_seconds: f32) {
        self.audio_commands_mut().set_emitter_volume(emitter, volume, fade_seconds);
    }

    fn set_emitter_panning(&mut self, emitter: &str, panning: f32, fade_seconds: f32) -> Result<()>{
        let id=self.audio_emitter_id(emitter)?;
        self.audio_commands_mut().set_emitter_panning(id, panning, fade_seconds);

        Ok(())
    }
    fn set_emitter_panning_from_id(&mut self, emitter: AudioEmitterId, panning: f32, fade_seconds: f32) {
        self.audio_commands_mut().set_emitter_panning(emitter, panning, fade_seconds);
    }
}

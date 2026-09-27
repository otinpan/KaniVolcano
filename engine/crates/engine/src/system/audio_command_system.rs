use anyhow::{Result, anyhow};
use kani_volcano_audio::{
    AudioSystem, AudioBusDescriptor, AudioBusKind, EmitterSettings
};
use crate::{
    Resources, AudioCommand, AudioCommandQueue, AudioBusSettings, AudioBusType,
    AudioEmitterSettings, AudioPlaybackResource,
};

#[derive(Clone, Debug, Default)]
pub struct AudioCommandSystem;

impl AudioCommandSystem{
    pub fn run(
        &mut self,
        commands: &mut AudioCommandQueue,
        resources: &mut Resources,
        system: &mut AudioSystem,
    ) -> Result<()>{
        for command in commands.drain() {
            if let Err(error) = self.apply_command(command, resources, system) {
                log::error!("audio command failed: {error:#}");
            }
        }

        let finished = system.collect_finished();
        resources.remove_finished_audio_playbacks(&finished);
        Ok(())
    }

    fn apply_command(
        &mut self,
        command: AudioCommand,
        resources: &mut Resources,
        system: &mut AudioSystem,
    ) -> Result<()> {
            match command{
                AudioCommand::CreateBus { name, settings } =>{
                    anyhow::ensure!(resources.audio_bus_id(&name).is_none(),
                        "audio bus already registered: {name}");
                    let descriptor=bus_settings_to_descriptor(settings, &resources)?;
                    let bus_handle=system.create_bus(descriptor)?;
                    resources.register_audio_bus(name.as_str(),bus_handle)?;
                },
                AudioCommand::CreateEmitter { name, settings } => {
                    anyhow::ensure!(resources.audio_emitter_id(&name).is_none(),
                        "audio emitter already registered: {name}");
                    let emitter_settings=engine_emitter_settings_to_system_emitter_settings(settings, &resources)?;
                    let emitter_handle=system.create_emitter(emitter_settings)?;
                    resources.register_audio_emitter(name.as_str(),emitter_handle)?;
                },
                AudioCommand::Play {
                    playback,
                    audio,
                    settings,
                } => {
                    anyhow::ensure!(
                        matches!(
                            resources.audio_playback(playback),
                            Some(AudioPlaybackResource::Pending)
                        ),
                        "audio playback is not pending: {playback:?}"
                    );

                    let result = (|| {
                        let audio_handle = resources
                            .get_audio_handle(audio)
                            .ok_or_else(|| anyhow!("audio not found: {audio:?}"))?;

                        system.play(audio_handle, settings)
                    })();

                    match result {
                        Ok(handle) => {
                            resources.activate_audio_playback(playback, handle)?;
                        }
                        Err(error) => {
                            resources.remove_audio_playback(playback);
                            return Err(error);
                        }
                    }
                }
                AudioCommand::PlayOnBus { playback, audio, bus, settings } =>{
                    anyhow::ensure!(
                        matches!(
                            resources.audio_playback(playback),
                            Some(AudioPlaybackResource::Pending)
                        ),
                        "audio playback is not pending: {playback:?}"
                    );

                    let result = (|| {
                        let audio_handle = resources
                            .get_audio_handle(audio)
                            .ok_or_else(|| anyhow!("audio not found: {audio:?}"))?;
                        let bus_handle = resources
                            .get_audio_bus_handle(bus)
                            .ok_or_else(|| anyhow!("audio bus not found: {bus:?}"))?;

                        system.play_on_bus(audio_handle, bus_handle, settings)
                    })();

                    match result {
                        Ok(handle) => {
                            resources.activate_audio_playback(playback, handle)?;
                        }
                        Err(error) => {
                            resources.remove_audio_playback(playback);
                            return Err(error);
                        }
                    }
                }
                AudioCommand::PlayOnEmitter { playback, audio, emitter, settings } => {
                    anyhow::ensure!(
                        matches!(
                            resources.audio_playback(playback),
                            Some(AudioPlaybackResource::Pending)
                        ),
                        "audio playback is not pending: {playback:?}"
                    );

                    let result = (|| {
                        let audio_handle = resources
                            .get_audio_handle(audio)
                            .ok_or_else(|| anyhow!("audio not found: {audio:?}"))?;
                        let emitter_handle = resources
                            .get_audio_emitter_handle(emitter)
                            .ok_or_else(|| anyhow!("audio emitter not found: {emitter:?}"))?;

                        system.play_emitter(audio_handle, emitter_handle, settings)
                    })();

                    match result {
                        Ok(handle) => {
                            resources.activate_audio_playback(playback, handle)?;
                        }
                        Err(error) => {
                            resources.remove_audio_playback(playback);
                            return Err(error);
                        }
                    }
                }
                AudioCommand::Stop { playback, fade_seconds } => {
                    let handle = resources
                        .get_audio_playback_handle(playback)
                        .ok_or_else(|| anyhow!("audio playback not active: {playback:?}"))?;

                    system.stop(handle, fade_seconds)?;
                }
                AudioCommand::Pause { playback, fade_seconds } => {
                    let handle = resources
                        .get_audio_playback_handle(playback)
                        .ok_or_else(|| anyhow!("audio playback not active: {playback:?}"))?;

                    system.pause(handle, fade_seconds)?;
                }
                AudioCommand::Resume { playback, fade_seconds } => {
                    let handle = resources
                        .get_audio_playback_handle(playback)
                        .ok_or_else(|| anyhow!("audio playback not active: {playback:?}"))?;

                    system.resume(handle, fade_seconds)?;
                }
                AudioCommand::SetPlaybackVolume { playback, volume, fade_seconds } =>{
                    let handle=resources
                        .get_audio_playback_handle(playback)
                        .ok_or_else(|| anyhow!("audio playback not active: {playback:?}"))?;

                    system.set_playback_volume(handle, volume, fade_seconds)?;
                }
                AudioCommand::SetBusVolume { bus, volume, fade_seconds } =>{
                    let handle=resources
                        .get_audio_bus_handle(bus)
                        .ok_or_else(||anyhow!("audio bus not found: {bus:?}"))?;

                    system.set_bus_volume(handle, volume, fade_seconds)?;
                }
                AudioCommand::SetBusReverb { bus, settings, fade_seconds } =>{
                    let handle=resources
                        .get_audio_bus_handle(bus)
                        .ok_or_else(||anyhow!("audio bus not found: {bus:?}"))?;
                    system.set_bus_reverb(handle, settings, fade_seconds)?;
                }
                AudioCommand::SetEmitterVolume { emitter, volume, fade_seconds } =>{
                    let handle=resources
                        .get_audio_emitter_handle(emitter)
                        .ok_or_else(|| anyhow!("audio emitter not found: {emitter:?}"))?;
                    system.set_emitter_volume(handle, volume, fade_seconds)?;
                }
                AudioCommand::SetEmitterPanning { emitter, panning, fade_seconds } =>{
                    let handle=resources
                        .get_audio_emitter_handle(emitter)
                        .ok_or_else(|| anyhow!("audio emitter not found: {emitter:?}"))?;
                    system.set_emitter_panning(handle, panning, fade_seconds)?;
                }
            }

        Ok(())
    }
}

fn bus_settings_to_descriptor(
    settings: AudioBusSettings,
    resources: &Resources,
) -> Result<AudioBusDescriptor>{
    let volume=settings.volume;
    let kind=match settings.bus_type{
        AudioBusType::Sub { output } => {
            let output_handle=resources.get_audio_bus_handle(output)
                .ok_or_else(|| anyhow!("audio bus not found: {output:?}"))?;
            AudioBusKind::Sub{
                output: output_handle,
            }
        },
        AudioBusType::Reverb{ settings } => AudioBusKind::Reverb{settings}
    };

    Ok(AudioBusDescriptor { volume, kind })
}


fn engine_emitter_settings_to_system_emitter_settings(
    settings: AudioEmitterSettings,
    resources: &Resources,
) -> Result<EmitterSettings> {
    let output = resources
        .get_audio_bus_handle(settings.output)
        .ok_or_else(|| {
            anyhow!("audio bus not found: {:?}", settings.output)
        })?;

    let sends = settings
        .sends
        .into_iter()
        .map(|send| {
            let bus = resources
                .get_audio_bus_handle(send.bus)
                .ok_or_else(|| {
                    anyhow!("audio send bus not found: {:?}", send.bus)
                })?;

            Ok(kani_volcano_audio::AudioSend {
                bus,
                gain: send.gain,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(EmitterSettings {
        output,
        volume: settings.volume,
        panning: settings.panning,
        sends,
    })
}

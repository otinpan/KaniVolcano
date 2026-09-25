use anyhow::{Result, ensure, anyhow};
use std::collections::{HashMap,HashSet};
use kira::{
    Decibels,
    AudioManager,
    AudioManagerSettings,
    DefaultBackend,
    sound::static_sound::{
        StaticSoundData,
        StaticSoundHandle,
    },
    effect::{
        reverb::{ReverbBuilder},
    },
    track::{
        TrackHandle, TrackBuilder, SendTrackBuilder,
    },
};

mod emitter;
mod audio;
mod bus;
pub use crate::emitter::{
    AudioEmitterHandle, EmitterSettings, AudioSend,
};
pub use crate::audio::{
    AudioHandle, DecodedAudio
};
pub use crate::bus::{
    AudioBusHandle, AudioBusDescriptor, AudioBusKind, make_tween, ReverbSettings,
    BusState, BusTrack,
};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaybackHandle(pub usize);


pub struct AudioSystem{
    manager: AudioManager<DefaultBackend>,

    assets: HashMap<AudioHandle, StaticSoundData>,
    buses:  HashMap<AudioBusHandle, BusState>,
    emitters: HashMap<AudioEmitterHandle, EmitterState>, // objects output sound
    playbacks: HashMap<PlaybackHandle, PlaybackEntry>, 

    next_audio_id: usize,
    next_emitter_id: usize,
    next_bus_id: usize,
}




struct EmitterState{
    track: TrackHandle,
}

struct PlaybackEntry{
    audio: AudioHandle,
    emitter: AudioEmitterHandle,
    sound: StaticSoundHandle,
}

impl AudioSystem{
    pub fn new() -> Result<Self> {
        let manager =
            AudioManager::<DefaultBackend>::new(
                AudioManagerSettings::default(),
            )?;

        let mut buses = HashMap::new();
        buses.insert(
            AudioBusHandle(0),
            BusState{
                track: BusTrack::Master,
                volume: 1.0,
                muted: false,
            },
        );


        Ok(Self {
            manager,
            assets: HashMap::new(),
            buses,
            emitters: HashMap::new(),
            playbacks: HashMap::new(),
            next_audio_id: 0,
            next_emitter_id: 0,
            next_bus_id: 1, // 0 is master bus
        })
    }

    // laod synchronoly
    pub fn load_audio(&mut self, path: &str) -> Result<AudioHandle> {
        let audio = DecodedAudio::from_file(path)?;
        self.register_audio(audio)
    }

    // only register
    pub fn register_audio(
        &mut self,
        audio: DecodedAudio,
    ) -> Result<AudioHandle> {
        let next = self.next_audio_id
            .checked_add(1)
            .ok_or_else(|| anyhow!("audio handle ID exhausted"))?;

        let handle = AudioHandle(self.next_audio_id);

        self.assets.insert(handle, audio.data);
        self.next_audio_id = next;

        Ok(handle)
    }

    pub fn unload_audio(&mut self, audio: AudioHandle) -> Result<()> {
        self.assets.remove(&audio)
            .ok_or_else(|| anyhow!("audio not found: {audio:?}"))?;

        Ok(())
    }

    // emitter ////////////////////
    pub fn create_emitter(
        &mut self,
        settings: EmitterSettings,
    ) -> Result<AudioEmitterHandle>{
        let next=self.next_emitter_id
            .checked_add(1)
            .ok_or_else(|| anyhow!("emitter handle ID exhausted"))?;

        // create tracker for emitter
        // this tracker do only send to other trackers role bus
        let mut builder=TrackBuilder::new()
            .volume(gain_to_decibels(settings.volume)?);

        let mut seen=HashSet::new();

        for send in &settings.sends{
            ensure!(
                seen.insert(send.bus),
                "duplocate send bus: {:?}",
                send.bus
            );

            let bus=self.buses.get(&send.bus)
                .ok_or_else(||anyhow!(
                    "send bus not found: {:?}",
                    send.bus
                ))?;

            let send_track=match &bus.track{
                BusTrack::Reverb{track, ..} => track,
                _ =>{
                    return Err(anyhow!(
                        "send destination must be a send track: {:?}",
                        send.bus
                    ));
                }
            };

            // register trackers to send
            builder=builder.with_send(
                send_track.id(),
                gain_to_decibels(send.gain)?,
            )
        }

        let output=self.buses.get_mut(&settings.output)
            .ok_or_else(|| anyhow!(
                "output bus not found: {:?}",
                settings.output
            ))?;

        let track=match &mut output.track{
            BusTrack::Master=>{
                self.manager.add_sub_track(builder)?
            }
            BusTrack::Sub(parent) =>{
                parent.add_sub_track(builder)?
            }
            BusTrack::Reverb{..} =>{
                return Err(anyhow!{
                    "a send track cannot be the normal output"
                });
            }
        };

        let handle=AudioEmitterHandle(self.next_emitter_id);

        self.emitters.insert(handle,EmitterState { track });
        self.next_emitter_id=next;

        Ok(handle)
    }

    // Bus //////////////////
    pub fn master_bus(&self) -> AudioBusHandle{
        AudioBusHandle(0)
    }

    pub fn create_bus(
        &mut self,
        descriptor: AudioBusDescriptor,
    ) -> Result<AudioBusHandle>{
        let next=self.next_bus_id
            .checked_add(1)
            .ok_or_else(|| anyhow!("bus handle ID exhausted"))?;

        let volume=gain_to_decibels(descriptor.volume)?;

        let track=match descriptor.kind{
            AudioBusKind::Sub{output} =>{
                let builder=TrackBuilder::new().volume(volume);

                let target=self.buses.get_mut(&output)
                    .ok_or_else(|| anyhow!(
                        "output bus not found: {output:?}"
                    ))?;
                
                let track=match &mut target.track{
                    BusTrack::Master=>{
                        self.manager.add_sub_track(builder)?
                    }
                    BusTrack::Sub(target) =>{
                        target.add_sub_track(builder)?
                    }
                    BusTrack::Reverb{..} =>{
                        return Err(anyhow!(
                            "a send bus cannot be a normal output"
                        ));
                    }
                };

                BusTrack::Sub(track)
            }
            AudioBusKind::Reverb{settings} =>{
                settings.validate()?;
                let mut builder=SendTrackBuilder::new().volume(volume);

                let effect=builder.add_effect(
                    ReverbBuilder::new()
                        .feedback(settings.feedback)
                        .damping(settings.damping)
                        .stereo_width(settings.stereo_width)
                        .mix(1.0),
                );

                let track=self.manager.add_send_track(builder)?;

                BusTrack::Reverb{track,effect}
            }
        };

        let handle=AudioBusHandle(self.next_bus_id);
        self.buses.insert(handle, BusState {
            track,
            volume: descriptor.volume,
            muted: false,
        });
        self.next_bus_id=next;

        Ok(handle)
    }

    pub fn set_bus_volume(
        &mut self,
        bus: AudioBusHandle,
        volume: f32,
        transition_seconds: f32,
    ) -> Result<()>{
        gain_to_decibels(volume)?;
        let tween=make_tween(transition_seconds)?;

        let state=self.buses.get_mut(&bus)
            .ok_or_else(|| anyhow!("bus not found: {bus:?}"))?;

        state.volume = volume;
        self.apply_bus_volume(bus, tween)
    }

    fn apply_bus_volume(&mut self, bus: AudioBusHandle, tween: kira::Tween) -> Result<()> {
        let state = self.buses.get_mut(&bus)
            .ok_or_else(|| anyhow!("bus not found: {bus:?}"))?;
        let volume = if state.muted {
            Decibels::SILENCE
        } else {
            gain_to_decibels(state.volume)?
        };

        match &mut state.track{
            BusTrack::Master =>{
                self.manager.main_track().set_volume(volume, tween);
            }
            BusTrack::Sub(track) =>{
                track.set_volume(volume, tween);
            }
            BusTrack::Reverb{track, ..} =>{
                track.set_volume(volume,tween);
            }
        }
        Ok(())
    }

    pub fn set_bus_muted(
        &mut self,
        bus: AudioBusHandle,
        muted: bool,
    ) -> Result<()>{
        let tween = make_tween(0.01)?;
        let state = self.buses.get_mut(&bus)
            .ok_or_else(|| anyhow!("bus not found: {bus:?}"))?;
        if state.muted == muted {
            return Ok(());
        }
        state.muted = muted;
        self.apply_bus_volume(bus, tween)
    }

    pub fn set_bus_reverb(
        &mut self,
        bus: AudioBusHandle,
        settings: ReverbSettings,
        transition_seconds: f32,
    ) -> Result<()>{
        settings.validate()?;
        let tween=make_tween(transition_seconds)?;

        let state=self.buses.get_mut(&bus)
            .ok_or_else(|| anyhow!("bus not found: {bus:?}"))?;

        let BusTrack::Reverb{effect, ..} = &mut state.track else{
            return Err(anyhow!("bus has no reverb: {bus:?}"));
        };

        effect.set_feedback(settings.feedback, tween);
        effect.set_damping(settings.damping, tween);
        effect.set_stereo_width(settings.stereo_width, tween);

        Ok(()) 
    }

    // player ////////////////////
    /*pub fn play(
        &mut self,
        audio: AudioHandle,
        settings: PlaybackSettings
    ) -> Result<PlayBackHandle>{

    }

    pub fn play_emitter(
        &mut self,
        audio: AudioHandle,
        emitter: AudioEmitterHandle,
    ) -> Result<PlaybackHandle>{

    }

    pub fn play_on_bus(
        &mut self,
        audio: AudioHandle,
        bus: AudioBusHandle,
        settings: PlaybackSettings
    ) -> Result<PlaybackHandle>{

    }

    pub fn stop(
        &mut self,
        playback: PlaybackHandle,
        fade_seconds: f32, 
    ) -> Result<()>{
        Ok(())
    }

    pub fn pause(
        &mut self,
        playback: PlaybackHandle,
        fade_seconds: f32,
    ) -> Result<()>{
        Ok(())
    }

    pub fn resume(
        &mut self,
        playback: PlaybackHandle,
        fade_seconds: f32,
    ) -> Result<()>{
        Ok(())
    }

    pub fn set_playback_volume(
        &mut self,
        playback: PlaybackHandle,
        volume: f32,
        transition_seconds: f32,
    ) -> Result<()>{

    }

    pub fn playback_state(&self, playback: PlaybackHandle) -> Option<PlaybackState>{

    }

    // finish /////////////////////////
    pub fn collect_finished(&mut self) -> Vec<PlaybackHandle>{

    }

    pub fn shutdown(&mut self) -> Result<()>{
        Ok(())
    }*/
}

pub fn gain_to_decibels(gain: f32) -> Result<Decibels> {
    ensure!(
        gain.is_finite() && gain >= 0.0,
        "gain must be finite and non-negative"
    );

    Ok(if gain == 0.0 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * gain.log10())
    })
}

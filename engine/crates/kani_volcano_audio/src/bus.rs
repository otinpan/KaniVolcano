use anyhow::{Result,ensure,anyhow};
use std::time::Duration;
use kira::{
    Tween,
    effect::reverb::{
        ReverbHandle,
    },
    track::{
        TrackHandle, SendTrackHandle,
    }
};
pub struct AudioBusDescriptor {
    pub volume: f32,
    pub kind: AudioBusKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AudioBusHandle(pub usize);


pub struct BusState{
    pub track: BusTrack,
    pub volume: f32,
    pub muted: bool,
}

pub enum BusTrack{
    Master,
    Sub(TrackHandle),
    Reverb{
        track: SendTrackHandle,
        effect: ReverbHandle,
    }
}

pub enum AudioBusKind {
    Sub {
        output: AudioBusHandle,
    },
    Reverb {
        settings: ReverbSettings,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ReverbSettings {
    pub feedback: f64,     // Controls how much of the reverberated signal is fed back into the effect.
    pub damping: f64,      // Controls how quickly high-frequency sounds are attenuated in the reverb.
    pub stereo_width: f64, // Controls the perceived width of the reverb in the stereo field.
}


impl ReverbSettings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.feedback.is_finite()
                && (0.0..1.0).contains(&self.feedback),
            "feedback must be between 0.0 and 1.0 (exclusive)"
        );

        ensure!(
            self.damping.is_finite()
                && (0.0..=1.0).contains(&self.damping),
            "damping must be between 0.0 and 1.0"
        );

        ensure!(
            self.stereo_width.is_finite()
                && (0.0..=1.0).contains(&self.stereo_width),
            "stereo_width must be between 0.0 and 1.0"
        );

        Ok(())
    }
}

pub fn make_tween(seconds: f32) -> Result<Tween> {
    let duration = Duration::try_from_secs_f32(seconds)
        .map_err(|_| anyhow!("invalid transition duration"))?;

    Ok(Tween {
        duration,
        ..Default::default()
    })
}
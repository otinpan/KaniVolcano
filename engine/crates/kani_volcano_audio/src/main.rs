use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, 
    sound::static_sound::StaticSoundData, 
    track::TrackBuilder,
    effect::filter::FilterBuilder,
    clock::ClockSpeed,
};
use std::time::Duration;
use anyhow::{Result};

const TEMPO: f64=120.0;
fn main() -> Result<()>{
    let mut manager=AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;

    let mut clock=manager.add_clock(ClockSpeed::TicksPerMinute(TEMPO))?;

    let mut track1=manager.add_sub_track({
        let mut builder=TrackBuilder::new();
        builder.add_effect(FilterBuilder::new().cutoff(1000.0));
        builder
    })?;
    let mut track2=manager.add_sub_track({
        let mut builder=TrackBuilder::new();
        builder.add_effect(FilterBuilder::new().cutoff(2000.0));
        builder
    })?;

    let sound_data1 = StaticSoundData::from_file("../../assets/sounds/sample_sound.mp3")?
        .start_time(clock.time()+2);
    let sound_data2=StaticSoundData::from_file("../../assets/sounds/sample_sound2.mp3")?
        .start_time(clock.time()+4);

    let mut sound1 = track1.play(sound_data1)?;
    let mut sound2=track2.play(sound_data2)?;

    sound1.set_playback_rate(
        1.0,
        kira::Tween {
            duration: Duration::from_secs(3),
            ..Default::default()
        },
    );
    sound2.set_playback_rate(
        1.0,
        kira::Tween{
            duration: Duration::from_secs(2),
            ..Default::default()
        }
    );
    clock.start();


    // Keep the audio manager alive while the backend plays the sound.
    println!("Playing sample_sound.mp3. Press Enter to exit.");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;

    Ok(())
}

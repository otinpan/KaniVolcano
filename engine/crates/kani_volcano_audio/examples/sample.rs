use kani_volcano_audio::{
    AudioSystem, AudioBusKind, AudioBusDescriptor, EmitterSettings,
    AudioSend,
};
use anyhow::{Result,anyhow};

fn main() -> Result<()>{
    let mut system=AudioSystem::new()?;

    // load audio
    let audio0=system.load_audio("../../assets/sounds/sample_sound.mp3")?;
    let audio1=system.load_audio("../../assets/sounds/sample_sound2.mp3")?;

    // create bus
    let mid_bus=system.create_bus(
        AudioBusDescriptor { 
            volume: 0.5,
            kind: AudioBusKind::Sub{output: system.master_bus()}
        }
    )?;

    let reverb_bus=system.create_bus(
        AudioBusDescriptor { 
            volume: 0.9,
            kind: AudioBusKind::Reverb { 
                settings: kani_volcano_audio::ReverbSettings { 
                    feedback: 0.9,
                    damping: 0.2, 
                    stereo_width: 0.9,
                }
            }}
    )?;

    // small_bus (0.3) -> mid_bus(0.5) -> master
    // output: 0.3 x 0.5 = 0.15
    let small_bus=system.create_bus(
        AudioBusDescriptor {
            volume: 0.3,
            kind: AudioBusKind::Sub{output: mid_bus}
        }
    )?;

    let large_bus=system.create_bus(
        AudioBusDescriptor { 
            volume: 1.0, 
            kind: AudioBusKind::Sub{output: mid_bus}
        }
    )?;

    // create emitter
    let emitter0=system.create_emitter(
        EmitterSettings{
            output: mid_bus,
            volume: 1.0,
            panning: -1.0,
            sends: vec![
                AudioSend{
                    bus: reverb_bus,
                    gain: 1.0,
                },
            ],
        }
    )?;

    let emitter1=system.create_emitter(
        EmitterSettings { 
            output: mid_bus, 
            volume: 1.0, 
            panning: 1.0, 
            sends: vec![
                AudioSend{
                    bus: reverb_bus,
                    gain: 1.0,
                },
            ] }
    )?;


    // play
    let playback0=system.play_emitter(
        audio0,
        emitter0,
        kani_volcano_audio::PlaybackSettings { 
            volume: 1.0, 
            looping: true
        }
    )?;
    
    println!("Playing sample_sound.mp3. Press Enter to pause.");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;

    system.pause(playback0,1.0)?;

    

    let playback1=system.play_emitter(
        audio1,
        emitter1,
        kani_volcano_audio::PlaybackSettings { 
            volume: 1.0,
            looping: true
        }
    )?;

    println!("Playing sample_sound2.mp3. Press Enter to exit.");
    line.clear();
    std::io::stdin().read_line(&mut line)?;
    Ok(())
}




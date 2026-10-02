# Audio API

## Overview of the Audio System

![](../../../assets/emitter_bus.png)

The Audio System used in this engine consists mainly of three concepts: **Audio**, which represents sounds such as BGM and sound effects; **Emitters**, which act as sound sources; and **Buses**, which define where audio signals are routed.

Audio is emitted from an Emitter, routed through one or more Buses, mixed together in the Master Bus, and finally output through speakers, headphones, or other audio devices.

This engine uses ECS. By attaching audio-related Components to Entities and playing audio, the Audio System handles sound playback.

```rust
pub struct AudioSource{
    pub audio: AudioAssetId,
    pub emitter: Option<AudioEmitterId>,
    pub settings: PlaybackSettings,
}
```

By attaching a Component like the one above to an Entity, you can give that Entity the ability to emit sound.

```rust
let play_emitter = context.play_on_emitter(
    "neko",
    "emitter0",
    PlaybackSettings {
        volume: 1.0,
        looping: true,
    },
);

let play_bus=context.play_on_bus(
    "bgm",
    "bgm_bus",
    PlaybackSettings { volume: 1.0, looping: true }
);

let play_master=context.play(
    "wind",
    PlaybackSettings { volume: 1.0, looping: false}
);
```

As shown above, audio can be played through an Emitter, directly through a Bus, or directly through the Master Bus.

You can choose whether to play audio from an Emitter attached to an Entity, through a specific Bus, or directly through the Master Bus.

## Creating Buses and Emitters

Let's take another look at the `AudioSource` Component.

```rust
pub struct AudioSource{
    pub audio: AudioAssetId,
    pub emitter: Option<AudioEmitterId>,
    pub settings: PlaybackSettings,
}
```

`AudioEmitterId` is the ID of an Emitter. It is obtained when an Emitter is created.

Let's look at how to create an Emitter.

```rust
fn create_emitter(&mut self, name: &str, settings: AudioEmitterSettings)
```

This is the API used to create an Emitter.

`name` specifies the name of the Emitter, and `settings` specifies its configuration.

```rust
pub struct AudioEmitterSettings{
    pub output: AudioBusId, // Main output Bus of the Emitter
    pub volume: f32, // Volume of the audio sent from the Emitter to the Bus
    pub panning: f32, // Panning of the audio sent from the Emitter to the Bus: -1.0 = left, 1.0 = right
    pub sends: Vec<AudioSend>, // Additional output Buses of the Emitter
}
```

```rust
pub struct AudioSend{
    pub bus: AudioBusId,
    pub gain: f32,
}
```

An Emitter must specify at least one output Bus using the `output` field.

If necessary, audio can also be sent to multiple additional Buses using the `sends` field.

Examples of audio sent through `sends` include reflected sound when audio hits a wall, diffraction effects, and other secondary sound effects.

By routing these signals to Buses specified in `sends`, more complex and expressive audio behavior can be created.

As shown here, creating an Emitter requires specifying a Bus, so the required Buses must normally be created beforehand.

However, if the Emitter only needs to send audio directly to the Master Bus, you do not need to create an additional Bus.

```rust
context.create_emitter(
    "output_master",
    AudioEmitterSettings{
        output: context.master_bus()?,
        volume: 1.0,
        panning: 0.0,
        sends: vec![],
    }
);
```

Now let's look at how to create a Bus.

```rust
fn create_bus(&mut self, name: &str, settings: AudioBusSettings)
```

```rust
pub struct AudioBusSettings{
    pub volume: f32,
    pub panning: f32,
    pub bus_type: AudioBusType,
}
```

```rust
pub enum AudioBusType{
    Sub{
        output: AudioBusId,
    },
    Reverb{
        settings: ReverbSettings,
    }
}
```

```rust
pub struct ReverbSettings {
    pub feedback: f64,     // Controls how much of the reverberated signal is fed back into the effect.
    pub damping: f64,      // Controls how quickly high-frequency sounds are attenuated in the reverb.
    pub stereo_width: f64, // Controls the perceived width of the reverb in the stereo field.
}
```

A Bus can be created using `create_bus()`.

As with an Emitter, you need to specify a name and configuration when creating a Bus.

In `AudioBusSettings`, `volume` controls the output volume of the Bus, `panning` controls the left-right balance, and `bus_type` specifies the type of Bus.

There are two Bus types: `Sub` and `Reverb`.

A `Sub` Bus receives audio from Emitters and routes it to another specified Bus.

A `Reverb` Bus receives audio from Emitters, generates reverberation, and outputs the processed audio.

A `Sub` Bus is a simple Bus that controls volume and panning, but Buses can be arranged hierarchically as shown below:

```text
SubBus0 -> SubBus1 -> ... -> MasterBus
```

The audio output from each Bus is affected by the `volume` and `panning` values of the Buses along the routing path.

A `Reverb` Bus generates reverberated audio.

The characteristics of the reverberation can be configured using `ReverbSettings`.

```text
ReverbBus0 -> MasterBus
```

A Reverb Bus outputs directly to the Master Bus.

### Bus Creation Example

```rust
context.create_bus(
    "bgm_bus",
    AudioBusSettings{
        volume: 1.0,
        panning: 0.0,
        bus_type: AudioBusType::Sub{
            output: context.master_bus()?,
        }
    }
)

context.create_bus(
    "water_bus",
    AudioBusSettings{
        volume: 1.0,
        panning: 0.0,
        bus_type: AudioBusType::Reverb{
            settings: ReverbSettings{
                feedback: 0.7,
                damping: 0.8,
                stereo_width: 0.5,
            }
        }
    }
)
```

### Emitter Creation Example

```rust
context.create_emitter(
    "emitter0",
    AudioEmitterSettings{
        output: context.audio_bus_id("bgm_bus")?,
        volume: 1.0,
        panning: 0.0,
        sends: vec![
            AudioSend{
                bus: context.audio_bus_id("water_bus")?,
                gain: 1.0,
            }
        ]
    }
)
```

The relationship between Emitters and Buses can be illustrated as follows:

![](../../../assets/audio_system.png)

## Loading Audio

```rust
pub struct AudioSource{
    pub audio: AudioAssetId,
    pub emitter: Option<AudioEmitterId>,
    pub settings: PlaybackSettings,
}
```

The `audio` field of the `AudioSource` Component must specify the ID of a loaded Audio asset.

Audio can be loaded synchronously from `app` using `load_audio()`, or asynchronously using `request_load_audio()`.

```rust
let audio_id=app.load_audio("neko", "assets/sounds/sample_sound.mp3")?;
```

```rust
let audio_id=context.request_load_audio("neko", "assets/sounds/sample_sound.mp3");
```

## Playback and Stopping

By attaching an `AudioSource` Component to an Entity, the Entity is given the ability to emit sound.

However, the audio will not start playing automatically.

Use `play()` or one of the playback APIs to start playback.

```rust
let play_emitter = context.play_on_emitter(
    "neko",
    "emitter0",
    PlaybackSettings {
        volume: 1.0,
        looping: true,
    },
);

let play_bus=context.play_on_bus(
    "bgm",
    "bgm_bus",
    PlaybackSettings { volume: 1.0, looping: true }
);

let play_master=context.play(
    "wind",
    PlaybackSettings { volume: 1.0, looping: false}
);
```

These functions return a `PlaybackId`, which identifies a single playback instance.

To stop playback, use `stop()` and specify the corresponding `PlaybackId`.

```rust
fn stop(&mut self, playback: AudioPlaybackId, fade_seconds: f32)
```

```rust
context.stop(play_emitter,0.4)
```

`fade_seconds` specifies the amount of time, in seconds, over which the audio fades out before stopping.

Pausing and resuming work in the same way.

```rust
fn pause(&mut self, playback: AudioPlaybackId, fade_seconds: f32) 
fn resume(&mut self, playback: AudioPlaybackId, fade_seconds: f32) 
```

## Changing Emitter and Bus Settings

You may want to dynamically adjust volume and panning depending on the positional relationship between the player and an object that emits sound, such as an Entity with an `AudioSource`.

You can change these values using APIs such as `set_bus_volume()` and `set_emitter_panning()`.

Changes are immediately applied to the corresponding playback instances.

```rust
fn set_playback_volume(
    &mut self,
    playback: AudioPlaybackId,
    volume: f32,
    fade_seconds: f32
)

fn set_bus_volume(
    &mut self,
    bus: &str,
    volume: f32,
    fade_seconds: f32
) -> Result<()>

fn set_bus_volume_from_id(
    &mut self,
    bus: AudioBusId,
    volume: f32,
    fade_seconds: f32
)

fn set_bus_panning(
    &mut self,
    bus: &str,
    panning: f32,
    fade_seconds: f32
) -> Result<()>

fn set_bus_panning_from_id(
    &mut self,
    bus: AudioBusId,
    panning: f32,
    fade_seconds: f32
)

fn set_bus_reverb(
    &mut self,
    bus: &str,
    settings: ReverbSettings,
    fade_seconds: f32
) -> Result<()>

fn set_bus_reverb_from_id(
    &mut self,
    bus: AudioBusId,
    settings: ReverbSettings,
    fade_seconds: f32
)

fn set_emitter_volume(
    &mut self,
    emitter: &str,
    volume: f32,
    fade_seconds: f32
) -> Result<()>

fn set_emitter_volume_from_id(
    &mut self,
    emitter: AudioEmitterId,
    volume: f32,
    fade_seconds: f32
)

fn set_emitter_panning(
    &mut self,
    emitter: &str,
    panning: f32,
    fade_seconds: f32
) -> Result<()>

fn set_emitter_panning_from_id(
    &mut self,
    emitter: AudioEmitterId,
    panning: f32,
    fade_seconds: f32
)
```

## Removing Emitters and Buses

Emitters and Buses that are no longer used should be explicitly removed.

Deleting an Entity does not automatically remove the Emitters or Buses associated with it.

Therefore, users must explicitly destroy them when they are no longer needed.

```rust
fn destroy_emitter(&mut self, emitter: &str) -> Result<()> 
fn destroy_emitter_from_id(&mut self, emitter: AudioEmitterId)

fn destroy_bus(&mut self, bus: &str) -> Result<()> 
fn destroy_bus_from_id(&mut self, bus: AudioBusId)
```

## Example

In this example, each Emitter is connected to multiple Buses.

The panning and volume of each Bus are dynamically changed depending on the positional relationship between the camera and the sound source.

### `audio_scene.rs`

```rust
use super::ChangeSceneCommand;
use crate::system::{AudioSystem, MoveRotateSystem};
use crate::component::{MoveRotateComponent};
use anyhow::{Result, anyhow};
use cgmath::{vec2, vec3};
use kani_volcano_audio::ReverbSettings;
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;
use std::collections::HashMap;
use winit::keyboard::KeyCode;

pub struct AudioScene {
    scene_id: SceneId,
    main_cube: Option<EntityId>,
    ground: f32,
    // emitter, output_bus, buses
    buses: HashMap<String, (String, Vec<String>)>,
}

impl Scene for AudioScene {
    fn name(&self) -> String {
        "AudioScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        self.scene_id = context.scene_id();
        context.set_skybox("default")?;
        self.create_camera(context)?;
        self.create_fundation(context)?;
        self.create_texts(context)?;
        self.create_cuboid(context)?;
        //self.load_audio(context)?;
        self.create_buses(context)?;

        // Names are available before the deferred audio registration completes.
        self.buses = (0..2)
            .map(|index| {
                (
                    format!("emitter{index}"),
                    (
                        format!("direct_{index}"),
                        ["water", "wood", "steel"]
                            .into_iter()
                            .map(|tag| format!("{tag}_{index}"))
                            .collect(),
                    ),
                )
            })
            .collect();

        self.add_update_systems(context);
        self.bind_input_commands(context);
        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let Some(cube) = self.main_cube else {
            return Ok(());
        };

        let Some(mesh_id) = context.mesh_asset_id(cube) else {
            return Ok(());
        };

        if !self.create_emitters(context)? {
            return Ok(());
        }

        if self.create_audio_objects(context, mesh_id)? {
            self.create_blocking_objects(context, mesh_id)?;
            self.play_audio(context)?;
            self.main_cube = None;
        }

        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        self.main_cube = None;

        // Emitters must be removed before the buses they reference.
        for index in 0..2 {
            let name = format!("emitter{index}");
            if context.audio_emitter_id(&name).is_ok() {
                context.destroy_emitter(&name)?;
            }
        }

        for index in 0..2 {
            for kind in ["direct", "water", "wood", "steel"] {
                let name = format!("{kind}_{index}");
                if context.audio_bus_id(&name).is_ok() {
                    context.destroy_bus(&name)?;
                }
            }
        }

        Ok(())
    }
}
```

The remaining implementation follows the same structure: Buses are created first, Emitters are connected to those Buses, and Entities with `AudioSource` Components use the Emitters to produce sound.

The example creates one direct Sub Bus and several Reverb Buses for each Emitter. The Reverb Buses represent different materials such as water, wood, and steel.

The audio system then calculates volume and panning from the relative positions of the camera and the sound source.

It also checks whether the line between the sound source and the camera intersects an object.

If an object blocks the sound, the Reverb Bus corresponding to the object's material is enabled, while the volume of the direct Bus is reduced.

For example:

```text
Sound Source
     |
     | Direct sound
     v
Direct Bus ----------------------> Master Bus

     |
     | Reflected / affected sound
     v
Water / Wood / Steel Reverb Bus -> Master Bus
```

This makes it possible to represent changes in sound depending on objects located between the listener and the sound source.

For example, if a steel object blocks the sound path, the steel Reverb Bus can become louder while the direct sound becomes quieter.

The Bus volume and panning are updated every frame using APIs such as:

```rust
context.set_bus_volume_from_id(bus, volume, 0.0);
context.set_bus_panning_from_id(bus, pan, 0.0);
```

This allows the audio routing and acoustic effects to dynamically respond to the positions of the camera, sound sources, and objects in the scene.
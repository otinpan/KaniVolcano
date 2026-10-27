# CHANGELOG
## ver0.1.0
2026/09/02
* relase demo version

## ver0.1.1
2026/09/05
### SceneCommandApi
Create `SceneCommandApi` to change scene from `UpdateContext` and `CommandContext`.  
example
```rust
pub struct ChangeSceneCommand {
    pub next_scene: String,
}

impl Command for ChangeSceneCommand {
    fn id(&self) -> String {
        format!("change_scene")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        context.set_current_scene(self.next_scene.as_str());
        Ok(())
    }
}
```

### Cuboid
Create `spawn_cuboid_3d()` in `ObjectApi`.

### `mesh_asset_id`
Create `mesh_asset_id()` in `EntityApi`. 

### Fixed Update System
Enable users register systems that are updated per fixed time step.

```rust
scene_context.add_fixed_update_system(name, system)
```

## ver0.2.0
### font
Users can use font in ver0.2.0
```rust
app.load_font("eng_font", r"C:\Windows\Fonts\arial.ttf")?;
```
```rust
let hello_world=context.spawn_text_ui2d(
    "eng_font", // font name
    "Hello\nWorld", // text
    100.0, // font size
    100.0, // line height
    vec2(0.0,0.0), // position
    vec2(1.0,2.0), // scale
    0.0, // rotation
    vec3(1.0,1.0,0.0), // color
    0.5, // alpha
);
```
```rust
let hello_world=context.spawn_text_3d(
    "eng_font", // font name
    "Hello\nWorld", // text
    100.0, // font size
    100.0, // line height
    vec3(-5.0,1.0,-1.0), // position
    vec3(0.01,0.01,0.01), // scale (to match world coordinate)
    vec3(0.0,0.0,0.0), // rotation
    vec3(1.0,1.0,1.0), // color
    0.5, // alpha
);
```

## ver0.2.1
Create `LoadAssetApi` which load assets using worker thread.
```rust
context.request_load_model(
    "viking_room_lit3d",
    "assets/models/viking_room.obj",
    PipelineKey::Lit3D,
    true,
);
```

## ver0.3.0
Create AudioSystem.
* Load audio
* Create bus
* Create emitter
* Play, Pause, Resume, Stop audio

By using emitters and buses, you can apply different panning and volume settings to the same audio source through separate signal paths.

## ver0.4.0
Create JobSystem
* Users can create jobs in systems
* Execute jobs in parallel using multiple threads
* Users can profile them

![](../assets/job_system_profiler.png)

## ver0.5.0
Create Debug Puase feature
* Users can pause/resume using TimeManagerApi
* Systems registerd in `add_fixed_update_system()` or commands  registerd in `bind_debubg_command()` can be affected.

## ver0.5.1
Fixed inconsistencies between vertex generation and index calculations in the sphere mesh.
* Corrected the loop ranges to prevent twisted geometry caused by incorrect vertex connections.
* Also standardized the sphere's coordinate system to Y-up.

## v0.5.2
Added a `DrawOrder` component that allows users to control the rendering order of UI elements using the `Ui2D` pipeline.

## v0.6.0
Change `Material` component.
```rust
pub struct Material {
    pub base_color: Vector4<f32>, // Base color (Supported)
    pub base_color_texture: Option<TextureHandle>, // Base color texture (Supported)

    // Surface properties
    pub metallic: f32, // Metallic factor (Supported)
    pub roughness: f32, // Surface roughness (Supported)
    pub metallic_roughness_texture: Option<TextureHandle>, // Per-pixel roughness (G) and metallic (B) values (Not supported)

    // Normal mapping (Not supported)
    pub normal_texture: Option<TextureHandle>, // Normal map for surface detail
    pub normal_scale: f32, // Normal map intensity

    // Emission
    pub emissive_color: Vector3<f32>, // Emissive color (Supported)
    pub emissive_strength: f32, // Emissive intensity multiplier (Supported)
    pub emissive_texture: Option<TextureHandle>, // Emissive texture (Not supported)

    // Ambient occlusion
    pub occlusion_strength: f32, // Controls the intensity of ambient occlusion (Not supported)

    // Transparency (Not supported)
    pub alpha_mode: AlphaMode, // Opaque, alpha masking, or alpha blending
    pub alpha_cutoff: f32, // Alpha threshold for Mask mode

    // Back-face rendering
    pub double_sided: bool, // Enables or disables back-face culling (Not supported)

    pub pipeline_key: PipelineKey,
}

pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}
```
![](../assets/material.png)
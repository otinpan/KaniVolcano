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
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
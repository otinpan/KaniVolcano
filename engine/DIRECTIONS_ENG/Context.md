# Context

KaniVolcano APIs are used through context objects. The context you receive depends on where the code is running.

A `Scene` uses `SceneContext` in `on_enter()` and `on_exit()`, and `UpdateContext` in `update()`.

```rust
impl Scene for TutorialScene {
    fn name(&self) -> String {
        "TutorialScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.set_skybox("ghost_skybox")?;
        let camera = context.spawn();
        context.add_component(camera, Transform::default());
        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.despawn_scene_owned_entities();
        Ok(())
    }
}
```

A `Command` uses `CommandContext` in `execute()`.

```rust
impl Command for CreateRectangleCommand {
    fn id(&self) -> String {
        "create_rectangle".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        context.spawn_rectangle_3d(
            vec3(0.0, 0.0, 0.0),
            0.3,
            0.3,
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 0.0),
            1.0,
            None,
            PipelineKey::Lit3D,
        )?;

        Ok(())
    }
}
```

## `SceneContext`

`SceneContext` is available from `Scene::on_enter()` and `Scene::on_exit()`.

It provides:

- Entity API
- Asset API
- Object API
- RenderCommand API
- Scene specific helpers

### `scene_id()`

Returns the `SceneId` of the current scene.

```rust
let scene_id: SceneId = context.scene_id();
```

### `despawn_scene_owned_entities()`

Despawns all entities that have `SceneOwned` with the current scene's `SceneId`.

```rust
let deleted_count: usize = context.despawn_scene_owned_entities();
```

### `bind_input_command()`

Binds a key, trigger, and command.

```rust
context.bind_input_command(
    KeyCode::Digit1,
    InputTrigger::Pressed,
    CreateRectangleCommand,
);
```

### `add_update_system()`

Registers an update system that runs every frame.

```rust
context.add_update_system("camera", CameraSystem);
```

### `add_fixed_update_system()`

Register an update system that runs at a fixed time step (default: 1/60 second).

```rust
context.add_fixed_update_system("physics", PhysicsSystem);
```


`SceneContext::spawn()` automatically adds `SceneOwned` to the created entity.

## `UpdateContext`

`UpdateContext` is available from `Scene::update()` and `UpdateSystem::update()`.

It provides:

- Entity API
- Object API
- Asset API
- Input API
- Time API
- RenderCommand API
- SceneCommand API

Use it for per-frame logic such as movement, animation, input checks, and mesh updates.

## `CommandContext`

`CommandContext` is available from `Command::execute()`.

It provides:

- Entity API
- Object API
- Asset API
- Input API
- RenderCommand API
- SceneCommand API

Use it for actions triggered by input, such as spawning or despawning objects.

Entities spawned from `CommandContext` are not automatically marked as scene-owned. If they should be removed when the scene exits, add `SceneOwned` manually.
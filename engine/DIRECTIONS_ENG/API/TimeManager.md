# TimeManagerApi

The `TimeManagementApi` trait is available through `App`, `UpdateContext`, and `CommandContext`.

```rust
use kani_volcano_engine::TimeManagementApi;

app.pause();        // Pause
app.step(1);        // Advance by one frame
app.resume();       // Resume
app.toggle_pause(); // Toggle between paused and running states
```

These APIs allow you to **pause**, **step through a specified number of frames**, and **resume** systems registered with `context.add_fixed_update_system("name", System)`, which are updated at a fixed rate.

For example:

```rust
context.add_update_system("camera", CameraSystem);
context.add_update_system(
    "audio",
    AudioSystem {
        buses: self.buses.clone(),
    },
);
context.add_fixed_update_system("rotator", MoveRotateSystem);
```

In this case, the time management APIs only affect `MoveRotateSystem`. The other systems continue to update normally.

## Binding Debug Commands to Keys

You can use `bind_debug_input_command` to bind a debug command to a specific key, allowing you to control pausing and resuming through keyboard input.

```rust
context.bind_debug_input_command(
    KeyCode::F6,
    InputTrigger::Pressed,
    DebugPauseCommand,
)
```

In this example, pressing `F6` toggles between the paused and running states.

- **Odd-numbered presses:** The rotating object stops.
- **Even-numbered presses:** The rotating object resumes moving.

`DebugPauseCommand` is provided by KaniVolcanoEngine and is implemented as follows:

```rust
pub struct DebugPauseCommand;

impl Command for DebugPauseCommand {
    fn id(&self) -> String {
        "debug_pause".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        context.toggle_pause();
        Ok(())
    }
}
```

Although this example uses a command to handle the pause/resume functionality, you can also implement the same behavior through a system.
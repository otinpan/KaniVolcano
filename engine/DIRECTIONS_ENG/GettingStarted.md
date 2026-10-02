# Getting Started

This guide walks through the first steps for using KaniVolcano Engine from another Rust project.

## Add Dependencies

Add KaniVolcano Engine and the required support crates to your `Cargo.toml`.

```toml
[dependencies]
anyhow = "1"
cgmath = "0.18"
pretty_env_logger = "0.5"
kani_volcano_engine = { package = "kani-volcano-engine", git = "https://github.com/otinpan/KaniVolcano.git" }
kani_volcano_math = { package = "kani-volcano-math", git = "https://github.com/otinpan/KaniVolcano.git" }
winit = "0.29"
```

## Create the Smallest App

Edit `main.rs`.

```rust
use anyhow::Result;
use kani_volcano_engine::prelude::*;

fn main() -> Result<()> {
    pretty_env_logger::init();

    run(
        EngineConfig {
            title: "KaniVolcano Tutorial".to_string(),
            width: 1024,
            height: 768,
        },
        |app| {
            Ok(())
        },
    )
}
```

The `run` function starts the engine.

The first argument is `EngineConfig`, which sets the window title, width, and height. The second argument is a closure that receives `&mut App`. This is where you load assets, create a skybox mesh, register scenes, and select the current scene.

At this point, the project should build.

```powershell
cargo build
```

If you run this minimal app immediately, you may see a panic like this:

```text
The vertical field of view cannot be below zero, found: 0.0 rad
```

This happens because no scene and no camera have been created yet. Next, create a scene and add a camera.

## Create a Scene

A scene represents one screen or stage of your game. Create `tutorial_scene.rs`.

```rust
use anyhow::Result;
use cgmath::vec3;
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;

pub struct TutorialScene {}

impl Scene for TutorialScene {
    fn name(&self) -> String {
        "TutorialScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let camera = context.spawn();

        context.add_component(
            camera,
            Transform {
                position: vec3(1.0, 0.0, 0.0),
                ..Default::default()
            },
        );

        context.add_component(
            camera,
            Camera {
                target: vec3(-5.0, 0.0, 0.0),
                up: vec3(0.0,0.0,1.0),
                fov_y: 45.0,
                near: 0.1,
                far: 100.0,
                yaw: std::f32::consts::PI,
                pitch: 0.0,
            },
        );

        Ok(())
    }

    fn update(&mut self, _context: &mut UpdateContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_exit(&mut self, _context: &mut SceneContext<'_>) -> Result<()> {
        Ok(())
    }
}
```

The `Scene` trait has four main methods:

- `name()`: returns the scene name
- `on_enter()`: called when the scene becomes active
- `update()`: called every frame while the scene is active
- `on_exit()`: called when the scene is changed

Register the scene in `main.rs`.

```rust
fn main() -> Result<()> {
    pretty_env_logger::init();

    run(
        EngineConfig {
            title: "Vulkan Tutorial".to_string(),
            width: 1200,
            height: 600,
        },
        |app| {
            app.add_scene(TutorialScene {})?;
            app.set_current_scene("TutorialScene")?;
            Ok(())
        },
    )
}
```

`add_scene()` registers the scene, and `set_current_scene()` makes it active.

![](../../assets/window.png)

## Draw Primitives

The window is now working, but the scene is still empty. Add some primitive objects in `on_enter()`.

```rust
fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
    self.create_camera(context)?;

    context.spawn_sphere_3d(
        vec3(-1.0, -0.7, 0.0),
        0.2,
        32,
        32,
        vec3(1.0, 0.0, 0.0),
        0.5,
        None,
        PipelineKey::Transparent3D,
    )?;

    context.spawn_cube_3d(
        vec3(-1.0, 0.0, 0.0),
        0.4,
        vec3(0.0, 0.0, 45.0),
        vec3(1.0, 1.0, 0.0),
        1.0,
        None,
        PipelineKey::DebugLine3D,
    )?;

    context.spawn_triangle_2d(
        vec2(0.5, 0.5),
        vec2(0.8, -0.2),
        vec2(0.4, -0.4),
        vec3(0.0, 0.0, 1.0),
        0.5,
        None,
    )?;

    Ok(())
}
```

![](../../assets/tutorial_primitives.png)

Primitive APIs let you specify position, size, color, alpha, texture, and pipeline.

## Load Models and Textures

Load assets in the `App` setup closure before registering the scene.

```rust
fn load_assets(app: &mut App) -> Result<()> {
    unsafe {
        app.load_model(
            "viking_room_lit3d",
            "assets/models/viking_room.obj",
            PipelineKey::Lit3D,
            false,
        )?;

        app.load_texture(
            "ghost",
            "assets/textures/ghost.png",
        )?;

        app.load_texture(
            "viking_room",
            "assets/textures/viking_room.png",
        )?;
    }

    Ok(())
}
```

Then spawn the model from the scene.

```rust
fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
    self.create_camera(context)?;
    self.create_primitives(context)?;

    let viking_texture = context
        .texture("viking_room")
        .unwrap_or(context.default_texture());

    context.spawn_model(
        "viking_room_lit3d",
        Transform {
            position: vec3(-3.0, 0.0, 0.0),
            rotation: vec3(0.0, 0.0, -45.0),
            scale: vec3(1.0, 1.0, 1.0),
        },
        Material {
            color: vec3(1.0, 1.0, 1.0),
            alpha: 1.0,
            use_texture: true,
            texture: viking_texture,
            pipeline_key: PipelineKey::Lit3D,
        },
    )?;

    Ok(())
}
```

![](../../assets/tutorial_model.png)

The `pipeline_key` used by the `Material` should match the pipeline used when the model was loaded. If they do not match, the object may not render correctly.

## Add a Skybox

Create the skybox mesh in `main.rs`.

```rust
|app| {
    app.create_skybox(20.0)?;
    load_assets(app)?;
    app.add_scene(TutorialScene {})?;
    app.set_current_scene("TutorialScene")?;
    Ok(())
}
```

Then load a skybox texture. A skybox texture needs six square images.

```rust
app.load_skybox_texture(
    "ghost_skybox",
    [
        "assets/textures/ghost.png",
        "assets/textures/ghost.png",
        "assets/textures/ghost.png",
        "assets/textures/ghost.png",
        "assets/textures/ghost.png",
        "assets/textures/ghost.png",
    ],
)?;
```

Set the skybox from the scene.

```rust
fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
    context.set_skybox("ghost_skybox")?;
    Ok(())
}
```

![](../../assets/tutorial_skybox.png)

## Move the Camera

KaniVolcano includes a default camera system. Add it from the scene.

```rust
fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
    self.create_camera(context)?;
    context.add_update_system("camera", CameraSystem);
    Ok(())
}
```

The camera system allows camera movement with keyboard and mouse input.

- `W`, `A`, `S`, `D`: move the camera
- Right mouse drag: rotate the camera

## Create a Component and System

Create a custom component.

```rust
use kani_volcano_engine::prelude::*;

pub struct MoveComponent {
    pub velocity: cgmath::Vector3<f32>,
}

impl Component for MoveComponent {}
```

Create a system that moves every entity with both `Transform` and `MoveComponent`.

```rust
use anyhow::Result;
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;

use crate::MoveComponent;

#[derive(Clone, Debug)]
pub struct MoveSystem;

impl UpdateSystem for MoveSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let delta_time = context.delta_seconds();

        for (_, transform, movement) in context.query2_mut_mut::<Transform, MoveComponent>() {
            let delta = movement.velocity * delta_time;
            transform.translate(delta);
        }

        Ok(())
    }
}
```

Attach the component to an entity.

```rust
let sphere = context.spawn_sphere_3d(
    vec3(-1.0, -0.7, 0.0),
    0.2,
    32,
    32,
    vec3(1.0, 0.0, 0.0),
    0.5,
    None,
    PipelineKey::Transparent3D,
)?;

context.add_component(
    sphere,
    MoveComponent {
        velocity: vec3(0.0, 0.2, 0.0),
    },
);
```

Register the system.

```rust
fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
    context.add_update_system("camera", CameraSystem);
    context.add_update_system("move", MoveSystem);
}
```

Now every entity with `Transform` and `MoveComponent` will move each frame.

## Create a Command

Commands are useful when you want input to trigger actions such as spawning objects.

```rust
struct CreateRectangleCommand {
    len: f32,
    color: Vec3,
    alpha: f32,
    texture: Option<&'static str>,
    pipeline_key: PipelineKey,
}

impl Command for CreateRectangleCommand {
    fn id(&self) -> String {
        "create_rectangle".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        let position = mouse_position_on_spawn_plane(context);

        context.spawn_rectangle_3d(
            position,
            self.len,
            self.len,
            vec3(0.0, 0.0, 0.0),
            self.color,
            self.alpha,
            self.texture,
            self.pipeline_key,
        )?;

        Ok(())
    }
}

fn mouse_position_on_spawn_plane(context: &CommandContext<'_>) -> Vec3 {
    let mouse = context.mouse_position();
    let window_size = context.window_size();
    let width = window_size.x.max(1.0);
    let height = window_size.y.max(1.0);
    let aspect = width / height;
    let world_height = 4.0;

    let x = mouse.x / width - 0.5;
    let y = 0.5 - mouse.y / height;

    vec3(-3.0, x * world_height * aspect, y * world_height)
}
```

Bind commands to input in the scene.

```rust
fn bind_input_commands(&mut self, context: &mut SceneContext<'_>) {
    context.bind_input_command(
        KeyCode::Digit1,
        InputTrigger::Pressed,
        CreateRectangleCommand {
            len: 0.2,
            color: vec3(1.0, 0.0, 0.0),
            alpha: 1.0,
            texture: None,
            pipeline_key: PipelineKey::Mesh3D,
        },
    );

    context.bind_input_command(
        KeyCode::Digit2,
        InputTrigger::Pressed,
        CreateRectangleCommand {
            len: 0.3,
            color: vec3(0.0, 1.0, 0.0),
            alpha: 0.5,
            texture: None,
            pipeline_key: PipelineKey::Transparent3D,
        },
    );

    context.bind_input_command(
        KeyCode::Digit3,
        InputTrigger::Pressed,
        CreateRectangleCommand {
            len: 0.1,
            color: vec3(1.0, 1.0, 1.0),
            alpha: 1.0,
            texture: Some("ghost"),
            pipeline_key: PipelineKey::Lit3D,
        },
    );

    context.bind_input_command(
        KeyCode::Digit4,
        InputTrigger::Pressed,
        CreateRectangleCommand {
            len: 0.2,
            color: vec3(0.0, 0.0, 1.0),
            alpha: 1.0,
            texture: None,
            pipeline_key: PipelineKey::DebugLine3D,
        },
    );
}
```

After registration:

- `Digit1` spawns a red rectangle with `Mesh3D`
- `Digit2` spawns a green transparent rectangle
- `Digit3` spawns a textured rectangle
- `Digit4` spawns a blue debug-line rectangle


## Complete Example

You can keep all code in one file while learning, but splitting it into separate files is easier once the scene grows.

`main.rs`

```rust
use anyhow::Result;
use kani_volcano_engine::prelude::*;
use kani_volcano_tutorial::TutorialScene;

fn main() -> Result<()> {
    pretty_env_logger::init();

    run(
        EngineConfig {
            title: "Vulkan Tutorial".to_string(),
            width: 1024,
            height: 768,
        },
        |app| {
            app.create_skybox(20.0)?;
            load_assets(app)?;
            app.add_scene(TutorialScene {})?;
            app.set_current_scene("TutorialScene")?;
            Ok(())
        },
    )
}

fn load_assets(app: &mut App) -> Result<()> {
    unsafe {
        app.load_model(
            "viking_room_lit3d",
            "assets/models/viking_room.obj",
            PipelineKey::Lit3D,
            false,
        )?;

        app.load_texture(
            "ghost",
            "assets/textures/ghost.png",
        )?;

        app.load_texture(
            "viking_room",
            "assets/textures/viking_room.png",
        )?;

        app.load_skybox_texture(
            "ghost_skybox",
            [
                "assets/textures/ghost.png",
                "assets/textures/ghost.png",
                "assets/textures/ghost.png",
                "assets/textures/ghost.png",
                "assets/textures/ghost.png",
                "assets/textures/ghost.png",
            ],
        )?;
    }

    Ok(())
}
```

`tutorial_scene.rs`

```rust
use anyhow::Result;
use cgmath::{vec2, vec3};
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;
use winit::keyboard::KeyCode;

use crate::{MoveComponent, MoveSystem};

type Vec3 = cgmath::Vector3<f32>;

pub struct TutorialScene {}

impl Scene for TutorialScene {
    fn name(&self) -> String {
        "TutorialScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.set_skybox("ghost_skybox")?;
        self.create_camera(context)?;
        self.create_primitives(context)?;
        self.bind_input_commands(context);
        self.create_models(context)?;
        self.add_update_systems(context);

        Ok(())
    }

    fn update(&mut self, _context: &mut UpdateContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        Ok(())
    }
}

impl TutorialScene {
    fn create_camera(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let camera = context.spawn();

        context.add_component(
            camera,
            Transform {
                position: vec3(0.0, 0.0, 0.0),
                ..Default::default()
            },
        );

        context.add_component(
            camera,
            Camera {
                target: vec3(-5.0, 0.0, 0.0),
                up: vec3(0.0,0.0,1.0),
                fov_y: 45.0,
                near: 0.1,
                far: 100.0,
                yaw: std::f32::consts::PI,
                pitch: 0.0,
            },
        );

        Ok(())
    }

    fn create_primitives(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let sphere = context.spawn_sphere_3d(
            vec3(-1.0, -0.7, 0.0),
            0.2,
            32,
            32,
            vec3(1.0, 0.0, 0.0),
            0.5,
            None,
            PipelineKey::Transparent3D,
        )?;

        context.add_component(
            sphere,
            MoveComponent {
                velocity: vec3(0.0, 0.2, 0.0),
            },
        );

        let triangle = context.spawn_triangle_2d(
            vec2(0.5, 0.5),
            vec2(0.8, -0.2),
            vec2(0.4, -0.4),
            vec3(0.0, 0.0, 1.0),
            0.5,
            None,
        )?;

        context.add_component(
            triangle,
            MoveComponent {
                velocity: vec3(0.0, 0.0, 0.2),
            },
        );

        Ok(())
    }

    fn create_models(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let viking_texture = context
            .texture("viking_room")
            .unwrap_or(context.default_texture());

        let viking_room = context.spawn_model(
            "viking_room_lit3d",
            Transform {
                position: vec3(-3.0, 0.0, 0.0),
                rotation: vec3(0.0, 0.0, -45.0),
                scale: vec3(1.0, 1.0, 1.0),
            },
            Material {
                color: vec3(1.0, 1.0, 1.0),
                alpha: 1.0,
                use_texture: true,
                texture: viking_texture,
                pipeline_key: PipelineKey::Lit3D,
            },
        )?;

        context.add_component(
            viking_room,
            MoveComponent {
                velocity: vec3(0.2, 0.0, 0.2),
            },
        );

        Ok(())
    }

    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("camera", CameraSystem);
        context.add_update_system("move", MoveSystem);
    }

    fn bind_input_commands(&mut self, context: &mut SceneContext<'_>) {
        context.bind_input_command(
            KeyCode::Digit1,
            InputTrigger::Pressed,
            CreateRectangleCommand {
                len: 0.2,
                color: vec3(1.0, 0.0, 0.0),
                alpha: 1.0,
                texture: None,
                pipeline_key: PipelineKey::Mesh3D,
            },
        );
    }
}

struct CreateRectangleCommand {
    len: f32,
    color: Vec3,
    alpha: f32,
    texture: Option<&'static str>,
    pipeline_key: PipelineKey,
}

impl Command for CreateRectangleCommand {
    fn id(&self) -> String {
        "create_rectangle".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        let position = mouse_position_on_spawn_plane(context);

        context.spawn_rectangle_3d(
            position,
            self.len,
            self.len,
            vec3(0.0, 0.0, 0.0),
            self.color,
            self.alpha,
            self.texture,
            self.pipeline_key,
        )?;

        Ok(())
    }
}

fn mouse_position_on_spawn_plane(context: &CommandContext<'_>) -> Vec3 {
    let mouse = context.mouse_position();
    let window_size = context.window_size();
    let width = window_size.x.max(1.0);
    let height = window_size.y.max(1.0);
    let aspect = width / height;
    let world_height = 4.0;

    let x = mouse.x / width - 0.5;
    let y = 0.5 - mouse.y / height;

    vec3(-3.0, x * world_height * aspect, y * world_height)
}
```

`move_component.rs`

```rust
use kani_volcano_engine::prelude::*;

pub struct MoveComponent {
    pub velocity: cgmath::Vector3<f32>,
}

impl Component for MoveComponent {}
```

`move_system.rs`

```rust
use anyhow::Result;
use kani_volcano_engine::prelude::*;
use kani_volcano_math::Transform;

use crate::MoveComponent;

#[derive(Clone, Debug)]
pub struct MoveSystem;

impl UpdateSystem for MoveSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let delta_time = context.delta_seconds();

        for (_, transform, movement) in context.query2_mut_mut::<Transform, MoveComponent>() {
            let delta = movement.velocity * delta_time;
            transform.translate(delta);
        }

        Ok(())
    }
}
```


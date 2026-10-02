# KaniVolcano Engine

![](../../assets/engine_screenshot.png)

**KaniVolcano Engine** is an open-source game engine that uses Vulkan as its graphics API and ECS as its core design pattern.

The goal of KaniVolcano is to hide much of Vulkan's low-level complexity while still giving users the flexibility of an ECS-based engine. Users can create entities, attach components, register systems, load assets, and draw interactive 2D/3D scenes without directly managing Vulkan objects.

At the current stage, KaniVolcano provides:

- Simple primitive rendering
- Model loading and rendering
- Entity, Component, and System creation through ECS
- Scene-based application structure
- Input command binding
- User-defined update systems

## Tutorial

`main.rs`

```rust
fn main() -> Result<()> {
    pretty_env_logger::init();

    run(
        EngineConfig {
            title: "Vulkan Tutorial (Rust)".to_string(),
            width: 1024,
            height: 768,
        },
        |app| {
            load_assets(app)?;
            app.create_skybox(20.0)?;
            app.add_scene(Basic3dScene::default())?;
            app.set_current_scene("Basic3dScene")?;
            Ok(())
        },
    )
}

fn load_assets(app: &mut App) -> Result<()> {
    unsafe {
        load_models(app)?;
        load_textures(app)?;
        load_skybox_textures(app)?;
    }
    Ok(())
}

unsafe fn load_models(app: &mut App) -> Result<()> {
    app.load_model(
        "viking_room",
        "assets/models/viking_room.obj",
        PipelineKey::Lit3D,
        false,
    )?;
    Ok(())
}

unsafe fn load_textures(app: &mut App) -> Result<()> {
    app.load_texture("viking_room", "assets/textures/viking_room.png")?;
    Ok(())
}

unsafe fn load_skybox_textures(app: &mut App) -> Result<()> {
    app.load_skybox_texture(
        "escapee",
        [
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
            "assets/textures/escapee.png",
        ],
    )?;
    Ok(())
}
```

`Basic3dScene.rs`

```rust
pub struct Basic3dScene {}

impl Scene for Basic3dScene {
    fn name(&self) -> String {
        "Basic3dScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.set_skybox("escapee")?;
        self.create_models(context)?;
        self.create_primitives(context)?;
        self.create_camera(context)?;
        self.add_update_systems(context);
        self.bind_input_commands(context);
        Ok(())
    }

    fn update(&mut self, _context: &mut UpdateContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.despawn_scene_owned_entities();
        Ok(())
    }
}

impl Default for Basic3dScene {
    fn default() -> Self {
        Self {}
    }
}

impl Basic3dScene {
    fn create_camera(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let camera = context.spawn();

        context.add_component(
            camera,
            Transform {
                position: vec3(0.0, 0.0, 3.0),
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

    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("rotator", RotatorSystem);
        context.add_update_system("camera", CameraSystem);
    }

    fn create_primitives(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        context.spawn_sphere_3d(
            vec3(-5.0, -1.0, 0.0),
            0.5,
            16,
            16,
            vec3(1.0, 0.0, 0.0),
            1.0,
            None,
            PipelineKey::Lit3D,
        )?;

        context.spawn_line_3d(
            vec3(0.0, -20.0, 0.0),
            vec3(0.0, 20.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            1.0,
        )?;

        Ok(())
    }

    fn create_models(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let texture = context
            .texture("viking_room")
            .unwrap_or(context.default_texture());

        context.spawn_model(
            "viking_room",
            Transform {
                position: vec3(-5.0, 0.0, 2.0),
                ..Default::default()
            },
            Material {
                color: vec3(1.0, 1.0, 1.0),
                alpha: 1.0,
                use_texture: true,
                texture,
                pipeline_key: PipelineKey::Lit3D,
            },
        )?;

        Ok(())
    }

    fn bind_input_commands(&mut self, context: &mut SceneContext<'_>) {
        context.bind_input_command(
            KeyCode::ArrowRight,
            InputTrigger::Pressed,
            SpawnVikingRoomCommand::default(),
        );

        context.bind_input_command(
            KeyCode::Digit1,
            InputTrigger::Pressed,
            CreateTriangleCommand {
                p0: vec3(0.0, 2.0, -0.3),
                p1: vec3(-7.0, 2.0, 0.3),
                p2: vec3(-2.0, 2.0, 1.0),
                color: vec3(1.0, 1.0, 0.0),
                alpha: 1.0,
                texture: Some("face"),
                pipeline_key: PipelineKey::Lit3D,
            },
        );
    }
}

struct CreateTriangleCommand {
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    color: Vec3,
    alpha: f32,
    texture: Option<&'static str>,
    pipeline_key: PipelineKey,
}

impl Command for CreateTriangleCommand {
    fn id(&self) -> String {
        "create_triangle".to_string()
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()> {
        context.spawn_triangle_3d(
            self.p0,
            self.p1,
            self.p2,
            self.color,
            self.alpha,
            self.texture,
            self.pipeline_key,
        )?;
        Ok(())
    }
}
```

This example creates a scene, loads assets, spawns objects, adds a camera, registers update systems, and binds input commands.

## What Is Vulkan?

Vulkan is a low-level, cross-platform graphics API defined by the Khronos Group.

Compared with higher-level graphics APIs, Vulkan gives the application more explicit control over the GPU. This can reduce CPU overhead, make rendering behavior more predictable, and give advanced users more room for optimization.

However, Vulkan also requires the developer to manage many details directly:

- Memory allocation
- Swapchain creation and management
- Shader and pipeline setup
- Descriptor sets
- Command buffer recording
- Synchronization

KaniVolcano wraps these low-level operations behind engine APIs so users can focus on building scenes, entities, components, systems, and shaders.

For example, instead of manually creating Vulkan buffers and command buffers, a user can write:

```rust
let polygon = context.spawn_polygon_3d(
    vec![
        vec3(-5.0, -0.4, -1.0),
        vec3(-5.0, -0.2, 0.0),
        vec3(-5.0, 0.5, -0.3),
        vec3(-5.0, 0.3, 0.2),
        vec3(-5.0, 0.0, 1.0),
        vec3(-5.0, -0.1, 1.2),
    ],
    vec3(0.0, 1.0, 0.0),
    1.0,
    Some("face"),
    PipelineKey::Lit3D,
)?;
```

This creates a polygon, selects the `Lit3D` pipeline, applies material data, and sends the required render work to the renderer.

## What Is ECS?

ECS stands for Entity Component System.

- Entity: an ID that represents an object
- Component: data attached to an entity
- System: logic that operates on entities with specific components

For example:

```rust
let camera = context.spawn();

context.add_component(camera, Transform::default());

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

context.add_update_system("rotator", RotatorSystem);
```

A system can then query the components it needs:

```rust
impl UpdateSystem for RotatorSystem {
    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        let delta_time = context.delta_seconds();

        for (_, transform, rotator) in context.query2_mut::<Transform, Rotator>() {
            transform.rotate(rotator.speed * delta_time);
        }

        Ok(())
    }
}
```

In object-oriented code, an object often owns all of its data:

```rust
struct Enemy {
    transform: Transform,
    health: Health,
    mesh: Mesh,
}
```

With ECS, the entity is only an ID, and each component type is stored separately:

```text
Transform:
[T0][T1][T2][T3][T4]...

Health:
[H0][H1][H2][H3][H4]...
```

This layout makes it easier for systems to process only the data they need. It can also improve cache efficiency because components of the same type are stored together.

## Future Direction

Planned areas include:

- Font rendering
- Sound support
- Collision detection through components and systems
- Basic physics through components and systems
- Shader extension APIs


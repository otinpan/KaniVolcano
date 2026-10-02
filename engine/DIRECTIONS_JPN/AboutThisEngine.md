# KaniVolcano Engine
![](../../assets/engine_screenshot.png)
**KaniVolcano Engine**はグラフィックスAPIとしてVulkanを使用し、デザインパターンとしてECSを採用した、オープンソースなゲームエンジンです。Vulkanの低レベルな重さを隠しつつ、ECS的な自由度を提供します。  
現段階では
* シンプルな図形の描画
* モデルのロード・描画
* ECSをベースとしたEntity、Component、Systemの作成と登録によるオブジェクトの更新

を提供しています。

## チュートリアル
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
            app.add_scene(Basic3dScene::default())?;
            app.set_current_scene("Basic3dScene")?;
            Ok(())
        },
    )
}

fn load_assets(app: &mut App) -> Result<()> {
    // load models
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
        PipelineKey::Mesh3D,
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
        context.set_skybox("ghost")?;
        self.create_models(context)?;
        self.create_primitives(context)?;
        self.create_camera(context)?;
        self.add_update_systems(context);
        self.bind_input_commands(context);
        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let despawned = context.despawn_scene_owned_entities();
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
        context.add_component(camera, Transform::default());
        let success = context.add_component(
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
        if !success {
            Err(anyhow!("failed to create Camera"))
        } else {
            Ok(())
        }
    }
    fn add_update_systems(&mut self, context: &mut SceneContext<'_>) {
        context.add_update_system("rotator", RotatorSystem);
        context.add_update_system("camera", CameraSystem);
    }

    fn create_primitives(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let ghost_texture = context
            .texture("ghost")
            .unwrap_or(context.default_texture());
        let sphere = context.spawn_sphere_3d(
            vec3(-5.0, -1.0, 0.0),
            0.5,
            16,
            16,
            vec3(1.0, 0.0, 0.0),
            1.0,
            None,
            PipelineKey::Lit3D,
        )?;

        let line = context.spawn_line_3d(
            vec3(0.0, -20.0, 0.0),
            vec3(0.0, 20.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            1.0,
        )?;

        Ok(())
    }

    fn create_models(&mut self, context: &mut SceneContext<'_>) -> Result<()> {
        let ghost_texture = context
            .texture("ghost")
            .unwrap_or(context.default_texture());

        let viking_room = context.spawn_model(
            "viking_room_lit3d",
            Transform {
                position: vec3(-5.0, 0.0, 2.0),
                ..Default::default()
            },
            Material {
                color: vec3(1.0, 0.0, 1.0),
                alpha: 1.0,
                use_texture: true,
                texture: ghost_texture,
                pipeline_key: PipelineKey::Lit3D,
            },
        );
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
        context.bind_input_command(KeyCode::Enter, InputTrigger::Pressed, DebugMonitor);
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
        format!("create_triangle")
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

少し長いかもしれませんが、オブジェクトやシステム、データを作成し、登録することで自由にインタラクト・描画することができます。

## Vulkanとは
Khronos Groupが策定した低レベル・クロスプラットフォーム対応のグラフィックスAPIです。OpenGLのような他のグラフィックスAPIと比べて、GPUに詳細な情報を伝えることが出来る反面、プログラマが明示的に記述する部分が多くなります。Vulaknが提供する低レベルAPIにより、
* 描画命令をGPUに送る際のCPU負荷の低減による、オーバーヘッドの削減
* 明示的なマルチスレッド対応
* クロスプラットフォーム対応

というメリットが期待されます。しかし、Vulkanでは
* メモリ管理
* Swapchain (描画するイメージ) の作成・管理
* シェーダ/パイプライン構築
* DescriptorSet (シェーダーが使うリソースを指定するセット) の構築
* コマンド作成・送信
* 同期処理

を扱わなければなりません。開発者が毎度これらを明示するのは大変です。そこで、KaniVolcanoはこれらの低レベルな命令の集合を抽象化しAPIとして提供します。
例えば、
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
のように書くことで、`Lit3D`というPipelineを選択し、環境光とスポットライトを反映するポリゴンを作成することができます。

## ECSとは
ECSとはEntity Component Systemの略です。
* Entity: 物体
* Component: 要素
* System: 法則

を別々に作成します。例えば、
```rust
// create new entity
let camera = context.spawn();

// grant entity a component (Transform)
context.add_component(camera, Transform::default());
// grant entity a component (Camera)
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

// enable a system (RotatorSystem)
context.add_update_system("rotator", RotatorSystem);
```
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
このようにEntityを作成し、事前に定義した`Transform`と`Camera` ComponentをEntityに対して付与します。最後に`RotatorSystem`を適用することで、`RotatorSystem`をもつ、すべてのEntityに対して中身の実装を適用することが出来ます。  
ECSはOOP (オブジェクト指向) に対して、パフォーマンスが優れる傾向があります。この畏友は、メモリ配置とキャッシュ効率です。OOPでは
```rust
struct Enemy {
    transform: Transform,
    health: Health,
    mesh: Mesh,
}
```
このように、1つのオブジェクトに様々なデータをまとめます。これらを更新する際は
```rust
for &mut enemy in enemies{
    enemy.transform.update();
}
```
みたいな感じになります。しかし各オブジェクトは
```
Enemy0: Transform Health AI Mesh
Enemy1: Transform Health AI Mesh
Enemy2: Transform Health AI Mesh
...
```
のように並んでいるため、CPUキャッシュには更新処理に使用しない`Health`や`Mesh`なども一緒に入りやすく、場所的局所性が低くなります。  
対してECSの場合、EntityはただのIDで、実体はComponentにあります。
```
Transform:
[T0][T1][T2][T3][T4]...

Health:
[H0][H1][H2][H3][H4]...
```
このように同じComponentが連続的に配置されているため
```rust
for transform in transforms {
    update(transform);
}
```
と順番に処理できます。連続したメモリの読み書きを可能にし、場所的局所性が高くなり、より効率的にキャッシュできます。

## 今後の方針
* フォント機能
* サウンド機能
* 当たり判定 (Component, System)
* 簡易物理 (Component, System)
* shader追加機能

他


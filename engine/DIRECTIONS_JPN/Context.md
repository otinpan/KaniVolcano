# Context
KaniVolcanoユーザーはContext経由で配布されるAPIを使用することが出来ます。
例えば、Appに登録するSceneは
```rust
impl Scene for TutorialScene{
    fn name(&self) -> String {
        "TutorialScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        context.set_skybox("ghost_skybox")?;
        let camera=context.spawn();
        context.add_component(
            camera,
            Transform{
                position: vec3(5.0,0.0,0.0),
                ..Default::default()
            }
        );
        context.add_component(
            camera,
            Camera{
                target: vec3(0.0,0.0,0.0),
                fov_y: 45.0,
                near: 0.1, // if objects are within `near`, they are not rendered.
                far: 100.0, // if objects are far than `far`, they are not rendered.
                yaw: std::f32::consts::PI, // horizontal way to move
                pitch: 0.0, // vertical way to move
            },
        );

        Ok(())
    }

    fn update(&mut self, context: &mut UpdateContext<'_>) -> Result<()>{
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        context.despawn_scene_owned_entities();
        Ok(())
    }

}
```

このように作れますが、Sceneトレイトでは`on_enter()`、`on_exit()`では`SceneContext`を`update()`では`UpdateContext`からAPIを呼びます。さらに、コマンドを作る場合は、
```rust
struct CreateRectangleCommand{
    len: f32,
    color: Vec3,
    alpha: f32,
    texture: Option<&'static str>,
    pipeline_key: PipelineKey,
    scene_id: SceneId,
}

impl Command for CreateRectangleCommand{
    fn id(&self) -> String{
        format!("create_rectangle")
    }

    fn execute(&self, context: &mut CommandContext<'_>) -> Result<()>{
        let position=mouse_position_on_spawn_plane(context);
        let rectangle=context.spawn_rectangle_3d(
            position,
            self.len,
            self.len,
            vec3(0.0,0.0,0.0),
            self.color,
            self.alpha,
            self.texture,
            self.pipeline_key,
        )?;
        context.add_component(
            rectangle,
            SceneOwned{
                scene_id: self.scene_id,
            }
        );
        Ok(())
    }
}
```
とコマンド実行時の処理`execute()`は`CommandContext`からAPIを呼びます。

ユーザーが触れることができるContextは
* `SceneContext` : Sceneトレイト
* `UpdateContext` : Sceneトレイト、UpdateSystemトレイト
* `CommandContext`: Commandトレイト

となります。

## `SceneContext`
`SceneContext`はSceneトレイトの`on_enter()`、`on_exit()`から使えます。`SceneContext`は以下のAPIを持ちます。
* EntityAPI
* AssetAPI
* ObjectAPI

そのほかに以下のAPIを持ちます。
### `scene_id()`
このSceneの`SceneId`を返します。
```rust
let scene_id: SceneId=context.scene_id();
```

### `despawn_scene_owned_entities()`
`SceneOwned`コンポーネントが付いていて、かつその`SceneId`がこのSceneのものと一致しているEntityをすべて削除します。
```rust
let deleted_num: usize=context.despawn_scene_owned_entities()
```

### `delete_scene_owned()`
Entityから`SceneOwned`コンポーネントを削除します
```rust
context.delete_scene_owned(entity)
```

### `bind_inpt_command()`
入力キーとトリガー、コマンドを結びつけます。
```rust
context.bind_input_command(
    KeyCode::Digit1,
    InputTrigger::Pressed,
    CreateRectangleCommand{
        len: 0.2,
        color: vec3(1.0,0.0,0.0),
        alpha: 1.0,
        texture: None,
        pipeline_key: PipelineKey::Mesh3D,
        scene_id: context.scene_id(),
    }
);
```

この場合、キー１を押したら、`CreateRectangleCommand`というユーザー定義のコマンドが呼ばれます。

### `add_update_system()`
名前とシステムを登録します。
```rust
context.add_update_system("camera",CameraSystem);
```
名前とユーザー定義のシステムを登録することで、毎フレーム呼ばれるSystemを作成することが出来ます。可変フレームで更新します。

### `add_fixed_update_system()`
```rust
context.add_fixed_update_system("physics", PhysicsSystem);
```

固定フレームで更新するようなシステムを追加します。ここで登録されたシステムは、デフォルトでは1/60秒ごとに更新されます。


### EntityAPI
EntityAPIを使うことが出来ます。ただ、EntityAPIで使える`spaw()`は`SceneContext`で呼ぶ場合は、自動的に`SceneOwned`が付与されます。


## `UpdateContext`
`Scene`の`update()`、Systemトレイトの`update()`は`UpdateContext`を使うことが出来ます。
* EntityAPI
* ObjectAPI
* AssetAPI
* InputAPI
* TimeAPI
* RenderCommandAPI
* SceneCommandAPI

を使用できます。

## `CommandContext`
Commandトレイトの`execute()`は`CommandContext`を使うことが出来ます。
* EntityAPI
* ObjectAPI
* AssetAPI
* InputAPI
* RenderCommandAPI
* SceneCommandAPI

を使用できます。

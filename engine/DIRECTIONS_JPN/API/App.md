# App API
Appから呼び出せるAPIの一覧です。
```rust
fn main()  -> Result<()>{
    pretty_env_logger::init();
    run(
        EngineConfig {
            title: "Vulkan Tutorial".to_string(), 
            width: 1024, 
            height: 768,
        },
        |app|{
            app.create_skybox(20.0)?;
            load_assets(app)?;
            app.add_scene(TutorialScene{})?;
            app.set_current_scene("TutorialScene")?;
            Ok(())
        }
    )
}
```

このように使います。
## Assets
モデルやテクスチャをAppからロードすることができます。  
### `obj`ファイルのロード
```rust
unsafe{
    // load model from assets/models
    app.load_model(
        "viking_room_lit3d",
        "assets/models/viking_room.obj",
        PipelineKey::Lit3D,
        false,
    )?;
}
```
1. モデルに付ける名前
2. モデルのパス
3. パイプラインの種類
4. モデルを自動的に解放するか

を指定します  

### `png`ファイルのロード
```rust
unsafe{
    // load textures from assets/textures/
    app.load_texture(
        "ghost",
        "assets/textures/ghost.png",
    )?;
}
```
1. テクスチャに付ける名前
2. テクスチャのパス

### Skybox
Skyboxとは背景に表示される立方体のことです。ワールドに立方体を置き、その内側に指定したテクスチャを張ることが出来ます。  
まず、テクスチャを張るための立方体を作成します。
```rust
app.create_skybox(20.0)?;
```
立方体の1辺の長さの半分の長さを指定します。この場合、1辺の長さは40.0になります。  
次に、Skyboxに張るテクスチャをロードします。この時点では、ただテクスチャをロードするだけで、Skyboxに張ることはできません。
```rust
unsafe{
    // load skybox texture.
    // need to select 6 textures
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
```

1. Skyboxに付ける名前
2. 各面にはる画像

## Scene
現段階ではSceneの管理はAppで行います。
### Sceneの作成
まずはSceneを作成する必要があります。Sceneは`Scene`トレイトを実装することで作成できます。
```rust
pub struct TutorialScene{}

impl Scene for TutorialScene{
    fn name(&self) -> String{
        "TutorialScene".to_string()
    }

    fn on_enter(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        Ok(())
    }

    fn on_update(&mut self, context: &mut  UpdateContext<'_>) -> Result<()>{
        Ok(())
    }

    fn on_exit(&mut self, context: &mut SceneContext<'_>) -> Result<()>{
        Ok(())
    }
}
```

* `name()`: Sceneの名前を登録
* `on_enter()`: Sceneに入ったときに呼ばれる関数
* `on_update()`: 毎フレーム呼ばれる関数
* `on_exit()`: Sceneから出るときに呼ばれる関数 

### Scene登録
作成したSceneはAppに登録する必要があります。
```rust
app.add_scene(TutorialScene{})?;
```
引数にはSceneのインスタンスを渡します。

### Scene切り替え
作成したSceneに切り替えるには、`set_current_scene()`を呼び出します。
```rust
app.set_current_scene("TutorialScene")?;
```

ここでは、Sceneで実装した`name()`で登録した名前を指定します。  

現段階では、SceneはAppからしか切り替えることが出来ません。今後は、`CommandContext`や`UpdateContext`からも切り替えられるように更新する予定です。

## 固定更新間隔の設定

固定更新の間隔を秒単位で指定します。デフォルトは1/60秒です。

```rust
app.set_fixed_delta_time(1.0 / 60.0)?;
```

ゼロ・負数・非有限値を指定するとエラーになります。


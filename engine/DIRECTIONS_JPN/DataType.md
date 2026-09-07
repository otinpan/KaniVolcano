# KaniVolcanoで使用されるデータ型
### `MeshAssetId`
`MeshAssetId`はKaniVolcanoが内部にもつ`MeshAsset`の識別子です。
```rust
pub struct MeshAsset {
    pub handle: MeshHandle,
    pub ref_count: usize,
    pub auto_release: bool,
}
```

`MeshAsset`は内部にVulkan側で使用するメッシュの識別子である、`MeshHandle`を持ちます。また、このメッシュをもつEntityをカウントする`ref_count`を持ちます。
`ref_count`が0かつ`auto_release`がtrueなら、自動的にこのメッシュはリリースされます。

### `FontAssetId`
`FontAssetId`は`FontAsset`の識別子です。
```rust
pub struct FontAsset{
    data: Vec<u8>,
}
```

### `PipelineKey`
`PipelineKey`は、どの描画パイプラインを使うかを指定するためのEnumです。
描画パイプラインはshaderと対応しており、頂点データの形式や描画方法を決めます。

- `Mesh3D`: 通常の3D描画に使います。ライトの影響は受けません。
- `Lit3D`: ライトの影響を受ける3D描画に使います。
- `Transparent3D`: 半透明の3D描画に使います。
- `DebugLine3D`: デバッグ用の線描画に使います。
- `Ui2D`: 画面上に表示する2D UI描画に使います。
- `Skybox`: Skybox描画に使う頂点レイアウトです。

### `VertexLayout`
`VertexLayout`は、描画パイプラインに渡す頂点データの形式を指定するためのEnumです。
位置、色、UV座標、法線など、頂点がどの情報を持つかを決めます。

基本的には`PipelineKey`から自動的に決まるため、通常は直接指定する場面は多くありません。
Meshを検索したり、基本図形のMeshを取得したりするときに使用します。
* `Mesh3D` : `PipelineKey::Mesh3D`、`PipelineKey::Transparent3D`に対応します。
* `Lit3D`: `PipelineKey::Lit3D`に対応します。
* `DebugLine3D`: `PipelineKey::DebugLine3D`に対応します。
* `Ui2D`: `PipelineKey::Ui2D`に対応します。
* `Skybox`: `PipelineKey::Skybox`に対応します。

`Pipeline::Mesh3D`と`Pipeline::Transparent3D`には同じ頂点データを送信しています。

`PipelineKey`の`required_vertex_layout()`メソッドから一意に`VertexLayout`を取得できます。
```rust
let vertex_layout: VertexLayout=PipelineKey::Transparent3D.required_vertex_layout()
```
```
-> VertexLayout::Mesh3D
```

### `PrimitiveType`
`PrimitiveType`は、基本図形の種類を表すEnumです。
基本図形のMeshを検索したり、Commandで生成する図形を指定したりするときに使います。

* `Triangle`: 三角形
* `Rectangle`: 四角形
* `Cube`: 立方体
* `Circle`: 円
* `Polygon`: 多角形
* `Sphere`: 球
* `Line`: 線

```rust
let mesh = context.primitive_asset_id(
    PrimitiveType::Cube,
    PipelineKey::Lit3D.required_vertex_layout(),
);
```

### `PrimitiveShape`
`PrimitiveShape`は、基本図形の具体的な形を表すEnumです。
`PrimitiveType`が図形の種類だけを表すのに対して、`PrimitiveShape`は頂点、半径、色などの形状データを持ちます。

```rust
PrimitiveShape::Rectangle {
    points: [
        vec3(0.0, -0.5, 0.5),
        vec3(0.0, -0.5, -0.5),
        vec3(0.0, 0.5, -0.5),
        vec3(0.0, 0.5, 0.5),
    ],
    color: vec3(1.0, 0.0, 0.0),
}
```

`update_primitive_mesh()`で既存の基本図形Meshを更新するときにも使います。

### `Material`
`Material`は、描画時の見た目を表すComponentです。
色、透明度、テクスチャを使うかどうか、使用する描画パイプラインを持ちます。

```rust
Material {
    color: vec3(1.0, 1.0, 1.0),
    alpha: 1.0,
    use_texture: false,
    texture: context.default_texture(),
    pipeline_key: PipelineKey::Lit3D,
}
```

* `color`: 色
* `alpha`: 透明度
* `use_texture`: テクスチャを使うかどうか
* `texture`: 使用するテクスチャ
* `pipeline_key`: 使用する描画パイプライン


### `InputTrigger`
`InputTrigger`は、キー入力がどの状態になったときにCommandを実行するかを表すEnumです。

* `Pressed`: キーが押された瞬間に実行
* `Down`: キーが押されている間、毎フレーム実行
* `Released`: キーが離された瞬間に実行

```rust
context.bind_input_command(
    KeyCode::Space,
    InputTrigger::Pressed,
    SpawnCommand,
);
```

### `SceneOwned`
`SceneOwned`は、そのEntityがどのSceneで作られたかを表すComponentです。
`SceneContext::spawn()`で作られたEntityには自動で付与されます。

```rust
SceneOwned {
    scene_id,
}
```

Scene終了時に`despawn_scene_owned_entities()`を呼ぶと、同じ`SceneId`を持つEntityをまとめて削除できます。

### `SceneId`
`SceneId`は、Sceneを識別するためのIDです。
`SceneOwned`の中に保存され、EntityがどのSceneに属しているかを判定するために使われます。

```rust
let scene_id = context.scene_id();
```

Command内で作成したEntityをScene終了時に削除したい場合は、`SceneOwned`を手動で追加するときに使います。

```rust
context.add_component(
    entity,
    SceneOwned {
        scene_id,
    },
);
```
# Asset API

ロード・登録済みの資産を扱うAPIです。`SceneContext`、`UpdateContext`、`CommandContext`から使用できます。

## モデル

名前からモデルのIDを取得します。見つからない場合はエラーになります。

```rust
let mesh: MeshAssetId = context.model_asset_id("viking_room_lit3d")?;
let model: GltfModelAssetId = context.gltf_model_asset_id("car")?;
```

## 基本図形

`PrimitiveType`と`VertexLayout`に一致する登録済みの基本図形を取得します。戻り値は`Option<MeshAssetId>`です。

```rust
let mesh = context.primitive_asset_id(PrimitiveType::Triangle, VertexLayout::Lit3D);
```

IDから基本図形の種類と頂点レイアウトも取得できます。該当する登録がない場合は`None`になります。

```rust
let primitive_type = context.primitive_type_from_asset_id(mesh_asset_id);
let vertex_layout = context.vertex_layout_from_asset_id(mesh_asset_id);
```

## テクスチャ

名前から`TextureAssetId`を取得します。見つからない場合はエラーになります。IDの取得だけでは参照数は増えません。

```rust
let texture: TextureAssetId = context.texture_asset_id("ghost")?;
```

Materialに設定する際、テクスチャが不要なら`None`を使用します。以下は、名前が見つからなければテクスチャなしにする例です。

```rust
let material = Material {
    base_color_texture: context.texture_asset_id("ghost").ok(),
    ..Material::default()
};
```

## Skybox

背景のSkyboxテクスチャを設定します。Skybox用meshが未登録ならエラー、指定したテクスチャ名が見つからなければデフォルトに切り替わります。

```rust
context.set_skybox("ghost_skybox")?;
let default_skybox: SkyboxTextureHandle = context.default_skybox_texture();
```

## Font

名前からフォントのIDを取得します。見つからない場合はエラーになります。

```rust
let font: FontAssetId = context.font_asset_id("font")?;
```

## Audio

音源・音源の配置を表すEmitter・音声をまとめるBusを、登録名から取得します。見つからない場合はエラーになります。

```rust
let audio: AudioAssetId = context.audio_asset_id("music")?;
let emitter: AudioEmitterId = context.audio_emitter_id("speaker")?;
let bus: AudioBusId = context.audio_bus_id("bgm")?;
```

これらはIDの取得のみで、再生は開始しません。

## 資産の一覧

各メソッドは、登録済み資産のIDと参照を返すiteratorです。

| メソッド | 要素 |
|---|---|
| `mesh_assets()` | `(MeshAssetId, &MeshAsset)` |
| `texture_assets()` | `(TextureAssetId, &TextureAsset)` |
| `font_assets()` | `(FontAssetId, &FontAsset)` |
| `audio_assets()` | `(AudioAssetId, &AudioAsset)` |
| `audio_emitters()` | `(AudioEmitterId, &AudioEmitterResource)` |
| `audio_buses()` | `(AudioBusId, &AudioBusResource)` |

```rust
for (id, mesh) in context.mesh_assets() {
    log::debug!("Mesh {id:?}: {mesh:?}");
}
```

## 明示的な解放

参照数が0の資産を、`auto_release`に関係なく登録解除します。GPU資産の破棄は、次のRenderSystem処理で行われます。

```rust
context.unload_texture(texture_id)?;
context.unload_mesh(mesh_id)?;
context.unload_gltf_model(model_id)?;
```

使用中・存在しないIDはエラーになり、登録は変更されません。デフォルトテクスチャと現在設定されているSkybox用meshも解放できません。

OBJモデルや基本図形には`unload_mesh()`を使用します。glTFモデルの解放では依存mesh・textureの参照を減らし、各資産の参照数と`auto_release`に従って解放します。共有中の資産や`auto_release=false`の依存資産は残ります。

使用中なら、先に`despawn()`や`set_material()`などで参照を外してください。自動解放済みの資産にunloadを呼ぶ必要はありません。解放後に再使用する場合は再ロードが必要です。

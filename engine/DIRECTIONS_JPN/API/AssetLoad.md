# AssetLoad API

モデルやテクスチャなどを非同期ロードするAPIです。ファイル読み込みなどのCPU処理はワーカースレッド、GPU転送とResourcesへの登録はメインスレッドで行います。

各requestは要求を追加して`()`を返します。完了フレームは保証されません。後続フレームで`model_asset_id()`や`texture_asset_id()`などが成功してから使用してください。ロード失敗はログに出力されます。同じ登録名の重複ロードは避けてください。

## Model

```rust
context.request_load_model(
    "viking_room_lit3d",
    "assets/models/viking_room.obj",
    PipelineKey::Lit3D,
    true,
);
```

## glTF / GLB

```rust
context.request_load_gltf_model("car", "assets/models/car.glb", true);
```

引数は登録名、`.gltf`または`.glb`のパス、`auto_release`です。`PipelineKey`は内部で決定します。

ファイル読み込みとglTF解析はワーカースレッドで行います。頂点変換・GPU転送・Material変換・登録はメインスレッドで行うため、大きなモデルでは登録時に時間がかかる場合があります。

後続フレームで`context.gltf_model_asset_id("car").is_ok()`なら登録済みです。`spawn_gltf_model("car", transform, None)?`でインスタンスを作成できます。毎フレーム生成しないよう、生成済みかどうかを保持してください。ロード失敗時はログを確認します。

## Texture

```rust
context.request_load_texture("jupiter", "assets/textures/jupiter.png", true);
```

Model・glTF/GLB・Textureの最後の引数は`auto_release`です。`true`なら最後の参照が外れたときに自動解放し、`false`なら保持します。未使用資産の明示解放は[Asset API](Asset.md)のunloadを使用します。名前からIDを取得するだけでは参照数は増えません。

## Skybox

```rust
context.request_load_skybox_texture("sky", [
    "assets/textures/sky_right.png",
    "assets/textures/sky_left.png",
    "assets/textures/sky_top.png",
    "assets/textures/sky_bottom.png",
    "assets/textures/sky_front.png",
    "assets/textures/sky_back.png",
]);
```

## Font

```rust
context.request_load_font("jpn_font", r"C:\Windows\Fonts\NotoSansJP-VF.ttf");
```

## Audio

```rust
context.request_load_audio("music", "assets/sounds/music.mp3");
```

Skybox・Font・Audioには`auto_release`引数はありません。

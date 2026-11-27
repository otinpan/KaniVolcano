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

## Texture

```rust
context.request_load_texture("jupiter", "assets/textures/jupiter.png", true);
```

ModelとTextureの最後の引数は`auto_release`です。`true`なら最後の参照が外れたときに自動解放し、`false`なら保持します。未使用資産の明示解放は[Asset API](Asset.md)のunloadを使用します。名前からIDを取得するだけでは参照数は増えません。

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

Skybox・Font・Audioには`auto_release`引数はありません。glTF/GLBの非同期ロードAPIはまだありません。

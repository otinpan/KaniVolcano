# AssetLoad API
非同期にモデルやテクスチャなどのアセットをロードするためのAPIです。  
ファイル読み込みや画像からピクセルの取得などはワーカースレッドで処理されます。
エンジンや描画エンジンへのアセットの登録は通常通りメインスレッドで処理されます。  
ロードされたアセットは次のフレームで使用することが出来ます。

## 非同期ロード
### Model
```rust
context.request_load_model(
    "viking_room_lit3d",
    "assets/models/viking_room.obj",
    PipelineKey::Lit3D,
    true,
);
```


### Texture
```rust
context.request_load_texture("jupiter", "assets/textures/jupiter.png");
```

### Skybox
```rust
context.request_load_skybox_texture("sky", "assets/textures/sky.png");
```

### Font
```rust
context.request_load_font("jpn_font", r"C:\Windows\Fonts\NotoSansJP-VF.ttf")?;
```

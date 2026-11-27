# AssetLoad API

Load models, textures, and other assets asynchronously. File I/O and CPU preparation run on worker threads; GPU upload and Resources registration run on the main thread.

Each request queues work and returns `()`. Completion is not guaranteed on the next frame. Wait until `model_asset_id()`, `texture_asset_id()`, or the corresponding lookup succeeds on a later frame before using the asset. Failures are logged. Avoid loading assets under an already registered name.

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

The last argument for Model and Texture is `auto_release`. `true` releases the asset when its last reference is removed; `false` keeps it loaded. Use the unload methods in [Asset API](Asset.md) to explicitly release unused assets. Looking up an ID by name does not retain it.

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

Skybox, Font, and Audio requests have no `auto_release` argument. Asynchronous glTF/GLB loading is not available yet.

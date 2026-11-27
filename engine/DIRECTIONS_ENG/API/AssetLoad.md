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

## glTF / GLB

```rust
context.request_load_gltf_model("car", "assets/models/car.glb", true);
```

Arguments are the registered name, the `.gltf` or `.glb` file path, and `auto_release`. The pipeline is selected internally.

File loading and glTF parsing run on the worker thread. Vertex conversion, GPU upload, Material conversion, and registration run on the main thread, so registering large models may still take noticeable time.

On a later frame, `context.gltf_model_asset_id("car").is_ok()` indicates that registration is complete. Create an instance with `spawn_gltf_model("car", transform, None)?`. Track whether it has been spawned to avoid creating an instance every frame. Check the logs if loading fails.

## Texture

```rust
context.request_load_texture("jupiter", "assets/textures/jupiter.png", true);
```

The last argument for Model, glTF/GLB, and Texture is `auto_release`. `true` releases the asset when its last reference is removed; `false` keeps it loaded. Use the unload methods in [Asset API](Asset.md) to explicitly release unused assets. Looking up an ID by name does not retain it.

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

Skybox, Font, and Audio requests have no `auto_release` argument.

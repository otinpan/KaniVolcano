# AssetLoad API

This API is used to asynchronously load assets such as models and textures.

File I/O and CPU-side processing, such as extracting pixel data from images, are performed on a worker thread.

Asset registration with the engine and renderer is performed on the main thread as usual.

Loaded assets can be used starting from the next frame.

## Asynchronous Loading

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
context.request_load_texture(
    "jupiter",
    "assets/textures/jupiter.png",
);
```

### Skybox

```rust
context.request_load_skybox_texture(
    "sky",
    "assets/textures/sky.png",
);
```

### Font

```rust
context.request_load_font(
    "jpn_font",
    r"C:\Windows\Fonts\NotoSansJP-VF.ttf",
)?;
```
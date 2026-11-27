# App API

App API is used during application setup. It is available from the closure passed to `run()`.

```rust
fn main() -> Result<()> {
    pretty_env_logger::init();

    run(
        EngineConfig {
            title: "KaniVolcano Tutorial".to_string(),
            width: 1024,
            height: 768,
        },
        |app| {
            app.create_skybox(20.0)?;
            load_assets(app)?;
            app.add_scene(TutorialScene {})?;
            app.set_current_scene("TutorialScene")?;
            Ok(())
        },
    )
}
```

Use `App` to load assets, create the skybox mesh, register scenes, and select the current scene.

## Asset Lifetime

Models and textures are uploaded to the GPU when loaded. References are held by entities and glTF model assets. With `auto_release=true`, an asset is released when its last reference is removed. Loading an asset or looking up its ID does not add an entity reference.

With `auto_release=false`, unused assets remain loaded. When their reference count is zero, use `unload_mesh()`, `unload_texture()`, or `unload_gltf_model()` from [Asset API](Asset.md). Reload released assets before using them again. Dependencies of an unloaded glTF model follow their own reference counts and `auto_release` flags.

## Loading Models

```rust
unsafe {
    app.load_model(
        "viking_room_lit3d",
        "assets/models/viking_room.obj",
        PipelineKey::Lit3D,
        false,
    )?;
}
```

Arguments:

1. Model name
2. Model file path
3. `PipelineKey`
4. `auto_release`

`PipelineKey` decides the vertex layout used by the model. `auto_release` decides whether the mesh should be released automatically when nothing uses it anymore.

## Loading glTF and GLB Models

```rust
unsafe {
    app.load_gltf_model(
        "teacup",
        "assets/models/teacup.glb",
        true,
    )?;
}
```

Arguments:

1. Model name
2. File path (`.gltf` or `.glb`)
3. Whether to release the model asset automatically (`auto_release`)

The pipeline is selected automatically. Use the registered name with `spawn_gltf_model()` to create an instance.

## Loading Textures

```rust
unsafe {
    app.load_texture("ghost", "assets/textures/ghost.png", true)?;
    app.load_texture_with_color_space(
        "face",
        "assets/textures/texture.png",
        TextureColorSpace::Linear,
        true,
    )?;
}
```

Arguments:

1. Texture name
2. Texture file path
3. `auto_release`

`load_texture_with_color_space()` takes `TextureColorSpace` as its third argument and `auto_release` as its fourth. `load_texture()` uses `TextureColorSpace::Srgb`, which automatically converts RGB values from sRGB to linear when sampled. `TextureColorSpace::Linear` uses the stored values without this conversion.

See [TextureColorSpace](../DataType.md#texturecolorspace) for common uses.

## Skybox

Create the skybox mesh first.

```rust
app.create_skybox(20.0)?;
```

The value is the half-size of the skybox cube.

Then load a skybox texture. A skybox texture requires six image paths.

```rust
unsafe {
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

### Font

You can load fonts into the engine. When rendering text, you need to specify a font that has been loaded in advance.

```rust
unsafe fn load_fonts(app: &mut App) -> Result<()> {
    app.load_font("eng_font", r"C:\Windows\Fonts\arial.ttf")?;
    app.load_font("jpn_font", r"C:\Windows\Fonts\NotoSansJP-VF.ttf")?;
    Ok(())
}
```


## Scene Management

Create a type that implements `Scene`, register it, and set it as the current scene.

```rust
app.add_scene(TutorialScene {})?;
app.set_current_scene("TutorialScene")?;
```

`set_current_scene()` uses the name returned by the scene's `name()` method.

## Setting the Fixed Update Interval

Set the fixed update interval in seconds. The default is 1/60 second.

```rust
app.set_fixed_delta_time(1.0 / 60.0)?;
```

Returns an error if the interval is zero, negative, or non-finite.

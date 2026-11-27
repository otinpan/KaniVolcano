# Asset API

Access assets that have already been loaded or registered. Available from `SceneContext`, `UpdateContext`, and `CommandContext`.

## Models

Get a model ID by name. A missing name returns an error.

```rust
let mesh: MeshAssetId = context.model_asset_id("viking_room_lit3d")?;
let model: GltfModelAssetId = context.gltf_model_asset_id("car")?;
```

## Primitive Meshes

Get a registered primitive matching `PrimitiveType` and `VertexLayout`. Returns `Option<MeshAssetId>`.

```rust
let mesh = context.primitive_asset_id(PrimitiveType::Triangle, VertexLayout::Lit3D);
```

You can also look up its primitive type and vertex layout by ID. Returns `None` if no matching registration exists.

```rust
let primitive_type = context.primitive_type_from_asset_id(mesh_asset_id);
let vertex_layout = context.vertex_layout_from_asset_id(mesh_asset_id);
```

## Textures

Get a `TextureAssetId` by name. A missing name returns an error. Looking up an ID does not increment its reference count.

```rust
let texture: TextureAssetId = context.texture_asset_id("ghost")?;
```

Use `None` in Material when no texture is needed. This example uses no texture if the name is missing.

```rust
let material = Material {
    base_color_texture: context.texture_asset_id("ghost").ok(),
    ..Material::default()
};
```

## Skybox

Set the background skybox texture. A missing skybox mesh returns an error; a missing texture name falls back to the default texture.

```rust
context.set_skybox("ghost_skybox")?;
let default_skybox: SkyboxTextureHandle = context.default_skybox_texture();
```

## Fonts

Get a font ID by name. A missing name returns an error.

```rust
let font: FontAssetId = context.font_asset_id("font")?;
```

## Audio

Look up a sound asset, an emitter representing a sound source's placement, or a bus grouping audio by its registered name. A missing name returns an error.

```rust
let audio: AudioAssetId = context.audio_asset_id("music")?;
let emitter: AudioEmitterId = context.audio_emitter_id("speaker")?;
let bus: AudioBusId = context.audio_bus_id("bgm")?;
```

These methods only retrieve IDs; they do not start playback.

## Asset Lists

Each method returns an iterator of registered asset IDs and references.

| Method | Item |
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

## Explicit Unloading

Unregister assets with zero references, regardless of `auto_release`. GPU destruction runs during the next RenderSystem stage.

```rust
context.unload_texture(texture_id)?;
context.unload_mesh(mesh_id)?;
context.unload_gltf_model(model_id)?;
```

An asset that is in use or missing returns an error without changing registrations. The default texture and the currently configured skybox mesh cannot be unloaded either.

Use `unload_mesh()` for OBJ models and primitives. Unloading a glTF model releases its ownership of meshes and textures. Dependencies are destroyed according to their own reference counts and `auto_release` flags. Shared dependencies and those with `auto_release=false` remain registered.

For assets in use, remove references first with `despawn()`, `set_material()`, or similar APIs. Assets already released automatically do not need an unload call. Reload an unloaded asset before using it again.

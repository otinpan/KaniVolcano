# Asset API

Asset API is used to access models, primitive meshes, textures, and skybox textures that have already been loaded or registered.

It is available from `SceneContext`, `UpdateContext`, and `CommandContext`.

## Models

Get the `MeshAssetId` of a loaded model.

```rust
let model: MeshAssetId = context.model_asset_id("viking_room_lit3d")?;
```

If the model exists, `Ok(MeshAssetId)` is returned. If it does not exist, an error is returned.

## Primitive Meshes

Get a primitive mesh by `PrimitiveType` and `VertexLayout`.

```rust
let mesh = context.primitive_asset_id(
    PrimitiveType::Triangle,
    VertexLayout::Lit3D,
);
```

This returns `Option<MeshAssetId>`.

You can also inspect a primitive mesh asset id.

```rust
let primitive_type = context.primitive_type_from_asset_id(mesh_asset_id);
let vertex_layout = context.vertex_layout_from_asset_id(mesh_asset_id);
```

## Textures

Get a texture by name.

```rust
let texture = context.texture("ghost")?;
```

If the texture is not found, an error is returned.

You can use the default texture as a fallback.

```rust
let texture = context.texture("ghost").unwrap_or(context.default_texture());
```

## Skybox

Set the current skybox texture.

```rust
context.set_skybox("ghost_skybox")?;
```

If the skybox mesh has not been created, or the texture name is not found, an error is returned.

You can also use the default skybox texture.

```rust
context.set_skybox(context.default_skybox_texture())?;
```

## Mesh Assets

Iterate through all mesh assets.

```rust
for (asset_id, mesh) in context.mesh_assets() {
    log::debug!("Mesh {asset_id:?}: {mesh:?}");
}
```

## Fontの取得
```rust
let font_assets: Iterator<Item = (FontAssetId, &FontAsset)> = context.font_assets();
```
Using `font_assets()`, you can retrieve all registered fonts.
```rust
let font_asset_id: Result<FontAssetId>= context.font_asset_id("font");
```
Using `font_asset_id(name)`, you can retrieve the FontAssetId of the font registered with the specified name. If no font with that name exists, an error is returned.
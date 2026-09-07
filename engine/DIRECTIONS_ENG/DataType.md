# Data Types Used in KaniVolcano

This page summarizes the main data types used by KaniVolcano APIs.

### `MeshAssetId`

`MeshAssetId` identifies a `MeshAsset` stored inside KaniVolcano.

```rust
pub struct MeshAsset {
    pub handle: MeshHandle,
    pub ref_count: usize,
    pub auto_release: bool,
}
```

A `MeshAsset` owns a `MeshHandle` used by the Vulkan renderer. `ref_count` tracks how many entities are using the mesh. If `auto_release` is `true`, the mesh is released when the reference count reaches zero.

### `FontAssetId`
`FontAssetId` identifies a `FontAsset`.
```rust
pub struct FontAsset{
    data: Vec<u8>,
}
```

### `PipelineKey`

`PipelineKey` selects which rendering pipeline to use. A pipeline decides the shader, vertex layout, and drawing method.

- `Mesh3D`: normal 3D rendering without lighting
- `Lit3D`: 3D rendering affected by lights
- `Transparent3D`: transparent 3D rendering
- `DebugLine3D`: line rendering for debugging
- `Ui2D`: 2D UI rendering
- `Skybox`: skybox rendering

### `VertexLayout`

`VertexLayout` describes the format of vertex data sent to a pipeline. It decides which data each vertex has, such as position, color, UV, and normal.

Usually this is decided automatically from `PipelineKey`.

- `Mesh3D`: used by `PipelineKey::Mesh3D` and `PipelineKey::Transparent3D`
- `Lit3D`: used by `PipelineKey::Lit3D`
- `DebugLine3D`: used by `PipelineKey::DebugLine3D`
- `Ui2D`: used by `PipelineKey::Ui2D`
- `Skybox`: used by `PipelineKey::Skybox`

```rust
let layout = PipelineKey::Transparent3D.required_vertex_layout();
// -> VertexLayout::Mesh3D
```

### `PrimitiveType`

`PrimitiveType` represents the type of a primitive shape.

- `Triangle`
- `Rectangle`
- `Cube`
- `Circle`
- `Polygon`
- `Sphere`
- `Line`

```rust
let mesh = context.primitive_asset_id(
    PrimitiveType::Cube,
    PipelineKey::Lit3D.required_vertex_layout(),
);
```

### `PrimitiveShape`

`PrimitiveShape` stores the actual data for a primitive, such as points, radius, segments, and color.

```rust
PrimitiveShape::Rectangle {
    points: [
        vec3(0.0, -0.5, 0.5),
        vec3(0.0, -0.5, -0.5),
        vec3(0.0, 0.5, -0.5),
        vec3(0.0, 0.5, 0.5),
    ],
    color: vec3(1.0, 0.0, 0.0),
}
```

It is also used by `update_primitive_mesh()`.

### `Material`

`Material` describes how an object looks when rendered.

```rust
Material {
    color: vec3(1.0, 1.0, 1.0),
    alpha: 1.0,
    use_texture: false,
    texture: context.default_texture(),
    pipeline_key: PipelineKey::Lit3D,
}
```

- `color`: base color
- `alpha`: transparency
- `use_texture`: whether to use a texture
- `texture`: texture handle
- `pipeline_key`: rendering pipeline

The `pipeline_key` must match the mesh's `VertexLayout`.

### `InputTrigger`

`InputTrigger` describes when a bound command should run.

- `Pressed`: run once when the key is pressed
- `Down`: run every frame while the key is held
- `Released`: run once when the key is released

```rust
context.bind_input_command(KeyCode::Space, InputTrigger::Pressed, SpawnCommand);
```

### `SceneOwned`

`SceneOwned` marks which scene created an entity. Entities created by `SceneContext::spawn()` receive it automatically.

```rust
SceneOwned { scene_id }
```

`despawn_scene_owned_entities()` uses this component to remove scene-owned entities when the scene exits.

### `SceneId`

`SceneId` identifies a scene.

```rust
let scene_id = context.scene_id();
```

Use it when manually adding `SceneOwned`, especially for entities created from commands.
# RenderCommand API

KaniVolcano does not directly execute Vulkan resource operations inside game logic. Mesh creation, mesh updates, mesh destruction, and skybox changes are queued as `RenderCommand`s and executed later by `RenderSystem`.

RenderCommand API is the API for issuing those render commands.

Primitive creation is provided by Object API, but functions such as `spawn_triangle_3d()` and `spawn_rectangle_3d()` also use render commands internally. They create an entity, attach the required components, and queue a command to create the GPU mesh.

## Updating Primitive Meshes

Use `update_primitive_mesh()` to change an existing primitive mesh.

```rust
let polygon_mesh3d: MeshAssetId = context
    .primitive_asset_id(PrimitiveType::Polygon, VertexLayout::Mesh3D)
    .ok_or_else(|| anyhow!("not found polygon mesh3d"))?;

context.update_primitive_mesh(
    polygon_mesh3d,
    PrimitiveShape::Polygon {
        points: vec![
            vec3(0.0, -0.7, 0.3),
            vec3(0.0, -0.4, 0.2),
            vec3(0.0, 0.7, 0.5),
            vec3(0.0, 0.2, -0.2),
            vec3(0.0, -0.5, -0.45),
        ],
        color: vec3(1.0, 0.0, 0.0),
    },
);
```

Here, `primitive_asset_id(PrimitiveType, VertexLayout)` finds the mesh whose primitive type is `Polygon` and vertex layout is `Mesh3D`. Then `update_primitive_mesh()` queues a command to update that mesh.

This updates the mesh itself. If several entities share the same mesh, all of them will change.

## MSAA Configuration

MSAA (Multisample Anti-Aliasing) is a rendering technique that smooths out jagged edges (also known as *aliasing*) that appear along diagonal lines and curved surfaces.

You can configure MSAA through the `RenderCommandApi`.

```rust
pub enum Msaa {
    X1,
    X2,
    X4,
    X8,
    X16,
    X32,
    X64,
}
```

### Setting MSAA

You can set the MSAA sample count from `App`:

```rust
// From App
app.set_msaa(Msaa::X4);
```

Or from a system, scene, or context:

```rust
// From a system, scene, or context
context.set_msaa(Msaa::X4);
```

Higher sample counts generally improve edge smoothness but increase VRAM usage and GPU workload, potentially resulting in lower FPS.

If an unsupported sample count is specified, the API returns an error and preserves the current MSAA setting.

By default, MSAA is set to the **highest sample count supported by the GPU for both color and depth attachments**.

### Getting the Current MSAA Setting

You can retrieve the current MSAA setting using `app.msaa()`:

```rust
let current_msaa: Msaa = app.msaa();
```

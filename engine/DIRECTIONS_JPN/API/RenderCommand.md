# RenderCommand API

KaniVolcanoでは、Vulkan側のリソースを操作する処理を、ゲームロジックの中で直接実行しません。
Meshの作成・更新・破棄やSkyboxの変更などは、いったんRenderCommandとしてキューに積まれ、RenderSystemによって別のタイミングで実行されます。

RenderCommand APIは、このRenderCommandを発行するためのAPIです。

基本図形の作成はObject APIが提供していますが、`spawn_triangle_3d()`や`spawn_rectangle_3d()`なども、内部ではRenderCommandを使っています。
これらの関数はEntityを作成し、必要なComponentを追加したうえで、GPU用Meshを作成する命令をRenderCommandQueueに積みます。

## 基本図形の更新
すでに作成された基本図形の形を変得たい場合、`update_primitive_mesh()`が有用です。
```rust
let polygon_mesh3d : MeshAssetId = context
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

ここでは、`context.primitive_asset_id(PrimtiveType, VertexLayout)`で`PrimitiveType`がPolygonで`VertexLayout`が`Mesh3D`のメッシュを探します。
見つけたメッシュに対して、新しく図形を更新します。

## MSAAの指定
MSAAとはグラフィックスの描画に置いて、物体の斜めや曲線の輪郭に生じる階段状のギザギザを滑らかにする技術のことです。RenderCommandApiからこれを指定することが出来ます。
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
```rust
// from app
app.set_msaa(Msaa::X4);
```

```rust
// from system, scene and context
context.set_msaa(Msaa::X4);
```

数値を上げるほど、VRAMとGPUの処理不可が増加し、FPSが低下しやすくなります。
非対応の値を指定した場合はエラーを返し、現在のMSAAの設定を維持するようにしています。

デフォルトでは、GPUカラー・深度の両方が対応する最大サンプル数です。

MSAAを取得するには`app.msaa()`を使用します。
```rust
let current_msaa: Msaa=app.msaa();
```
# Asset API
主にロードしたモデルやテクスチャを扱うためのAPIです。
## モデル
ロードしたモデルを取得することができます。
```rust
let model: ModelAssetId=context.model_asset_id("viking_room_lit3d")?;
```

モデルがある場合は、`Ok(ModelAssetId)`が返され、モデルがない場合は、`Err("model not found: {model_name})`が返されます。

## 基本図形
Triangle、Rectangle、Cube、Sphereなどのすでにロードされている基本図形も取得することが出来ます。しかし、これらの基本図形には名前のような、固有の識別子はありません。なので、AssetのAPIは、`PrimitiveType`と`VertexLayout`を指定して、それと一致した基本図形の1つの`MeshAssetId`を返します。
```rust
let triangle_lit3d: Option<MeshAssetId>=context.primitive_mesh_asset_id(PrimitiveType::Triangle, VertexLayout::Lit3D);
```

逆に`MeshAssetId`から、`PrimitiveType`と`VertexLayout`を取得することも出来ます。
```rust
let primitive_type: Option<PrimitiveType>=context.primitive_type_from_asset_id(mesh_asset_id);
```
```rust
let vertex_layout: Option<VertexLayout>=context.vertex_layout_from_asset_id(mesh_asset_id);
```

## テクスチャ
ロードしたテクスチャを取得することが出来ます。
```rust
let texture: Result<TextureHandle> = context.texture("ghost");
```
テクスチャがある場合は、`Ok(TextureHandle)`が返され、テクスチャがない場合は、`Err("texture not found: {texture_name})`が返されます。  

白紙のテクスチャを取得することもできます。
```rust
let default_texture: TextureHandle = context.default_texture();
```

```rust
let ghost_texture=context.texture("ghost").unwrap_or(context.default_texture());
```
とすることで、ghostという名前のテクスチャがない場合は、白紙のテクスチャを取得することが出来ます。


## Skybox
Skyboxとは背景に表示される立方体のことです。この立方体の内側に指定したテクスチャを張ることが出来ます。  
```rust
let is_skybox_texture: Result<()>=context.set_skybox("ghost_skybox");
```

もし、Skybox用のメッシュがまだ作成されていない場合は、`Err("skybox mesh is not registered")`が返されます。また、指定した名前のテクスチャがない場合は、`Err("skybox texture not found: {texture_name}")`が返されます。  

Texture同様、白紙のテクスチャを得ることもできます。
```rust
let skybox_texture: SkyboxTextureHandle=context.default_skybox_texture();
```

このようにすることで、白紙のテクスチャをSkyboxに張ることが出来ます。
```rust
let is_skybox_texture: Result<()>=context.set_skybox(context.default_skybox_texture());
```

## 便利関数
モデルと基本図形の両方の`(MeshAssetId, MeshAsset)`を取取得することが出来ます。
```rust
let mesh_assets=impl Iterator<Item=(MeshAssetId, MeshAsset)> = context.mesh_assets();
```
使用例
```rust
fn monitor_mesh_assets(&self, context: &mut CommandContext<'_>) -> Result<()> {
    for (asset_id, mesh) in context.mesh_assets() {
        log::debug!("Mesh {asset_id:?}: {mesh:?}");
    }

    Ok(())
}
```

## Fontの取得
```rust
let font_assets: Iterator<Item = (FontAssetId, &FontAsset)> = context.font_assets();
```
`font_assets()`では登録されたすべてのフォントを取得することが出来ます。
```rust
let font_asset_id: Result<FontAssetId>= context.font_asset_id("font");
```

`font_asset_id(name)`を使うと、`name`という名前で登録されたフォントが存在するなら`Ok(FontAssetId)`を、存在しないならエラーを返します。
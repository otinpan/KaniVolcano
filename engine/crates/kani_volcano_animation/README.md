# やりたいこと
* glTF、glbのロード。頂点・インデックス・ノード・スキン・アニメーションの保持
* 描画用の`MeshData<V>`への変換はengineで行う
* renderer_vulkanで`load_mesh_from_data`でGPUに転送、MeshHandleの取得
* 現在の`SourceMesh`は静的メッシュ向けで、関節番号やウェイトなどを保持できないため、これらのデータはkani_volcano_animationで必要なものを作る

* Scene
    - ModelInstanceComponent
    -  TransformComponent
* Nodes
    - ParentComponent
    - TransformComponent
    - MeshRendererComponent

* engine: AnimationSystem
    - 再生時刻
    - kani_volcano_animationにポーズ計算の依頼
    - ノード番号 -> EntityIdの対応表
    - 計算結果の反映
* engine: system
    - ワールド変換の計算
* engine: RenderSystem
    - RenderItemで送信


* `laod_gltf_model(path) -> GltfModel` でモデルのロード
    - ModelImageから画像のロード
        gltf::import
        → ModelImage（画素形式を保持）
        → engineでRGBA8へ変換
        → DecodedTexture
        → renderer.upload_decoded_texture(&decoded)
    - 画像登録
    - モデルのロード
    - モデル登録

* 参照カウンタの対応
    - `object_api`でspawnしたらgltf_modelの参照カウンタをインクリメントする
    - `entity_api`でdespawnしたらgltf_modelの参照カウンタをデクリメントする

* `despawn()`ではモデル資産の参照数を1減らして、0になったかを確認する
    - 子entityの削除し、各entityのメッシュ参照を`release_mesh`でリリース
    - ルートのcomponentから`GltrModelAssetId`を取得する
    - そのモデル資産の参照数を1減らす
    - 参照数が0かつ、モデルのauto_release==trueなら、`resources.remove_gltf_model_asset()`を呼ぶ
    - 戻り値のMeshHanleを描画コマンドに渡して、GPUメッシュを破棄する

* ロード途中の失敗時にGPU資産の回収
* テクスチャの個別解放



| 優先 | 機能 | 現在からの進め方 | 残りの重さ |
|---|---|---|---|
| 1 | PBRテクスチャの描画反映 | 専用descriptorを整備し、metallic-roughness → emissive・occlusion → normalの順に接続 | 中〜大 |
| 2 | ロード失敗時のGPU資産回収 | 工程をまたぐ失敗と、GPU作成途中の失敗を回収 | 中 |
| 3 | テクスチャ自動解放 | 資産の所有・共有、Material変更、descriptorの寿命を管理 | 大 |
| 4 | glTF, texture (color_space) 非同期ロード | CPU準備をバックグラウンド化し、既存の登録段階へ接続 | 中〜大 |
| 5 | ノードアニメーション→スキニング | まず再生・補間、その後GPUでの頂点変形 | 大 |

```
// PBR material descriptors. Base-color RGB is decoded by the sRGB image format.
// Metallic-roughness uses a linear image, with white as the absent-map fallback.
layout(set = 1, binding = 0) uniform sampler2D baseColorSampler;
layout(set = 1, binding = 1) uniform sampler2D metallicRoughnessSampler;
layout(set = 1, binding = 2) uniform sampler2D emissiveSampler;
layout(set = 1, binding = 3) uniform sampler2D occlusionSampler;
```

* `load_gltf_model`でgltfをロードし、その中にアニメーション用の頂点データが入っているならSkinnedVertexを採用、そうでないならLit3DVertexを採用
* PipelineKey::SkinnedPbr3D => VertexLayout::SkinnedVertexとし、専用のpipelineを作る
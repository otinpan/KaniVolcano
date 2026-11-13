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


# やりたいこと
* glTF、glbのロード。頂点・インデックス・ノード・スキン・アニメーションの保持
* 描画用の`MeshData<V>`への変換はengineで行う
* renderer_vulkanで`load_mesh_from_data`でGPUに転送、MeshHandleの取得
* 現在の`SourceMesh`は静的メッシュ向けで、関節番号やウェイトなどを保持できないため、これらのデータはkani_volcano_animationで必要なものを作る
# glTF / GLB・アニメーション実装計画

## 進め方

この計画は、ユーザーと一緒に設計・実装・動作確認を進めるためのものです。計画の作成は、全段階を一括で実装する指示ではありません。各段階で目的と変更範囲を説明し、ユーザーが実装する部分・一緒に検討する部分・Codexに依頼する部分を、その時点の会話に合わせて進めます。

まず小さいモデルで一つずつ動作を確認します。ユーザーの変更を確認してから次の作業に入り、未依頼の大規模な変更や次段階への自動移行は行いません。以下の型名・API名は設計案であり、実装時に確定します。

## 到達目標

- `.gltf` / `.glb`からモデル、マテリアル、ノード階層、スキン、アニメーションを読み込める。
- 静的モデルを表示した後、手動で関節を動かしてスキニングを確認できる。
- 任意時刻の姿勢をCPUで計算し、再生・ループ・一時停止・停止を制御できる。
- 同じモデルを複数配置しても、各個体が独立した姿勢と再生状態を持つ。
- ファイル由来のアニメーションだけでなく、KaniVolcanoEngineのSystemから姿勢を変更したり、コードでクリップを作成したりできる。

## 用語と読み込み対象

| 要素 | 扱う内容 |
| --- | --- |
| glTF | JSON形式の文書。外部のバッファ・画像を参照でき、埋め込みデータも扱える |
| GLB | JSONとバイナリをまとめるコンテナ。単一ファイルにまとめられるが、外部参照を持つ場合も考慮する |
| mesh / primitive | 頂点・インデックス・描画方式とマテリアル参照。1 meshに複数primitiveがある |
| material / texture | 色、画像、サンプラーなどの描画情報 |
| node | ローカル変換と親子階層。関節に限らず、メッシュや通常の配置ノードも含む |
| skin | 使用するjointのノード一覧とinverse bind matrix。jointの並びは階層の走査順とは限らない |
| animation | 時刻、補間方式、対象ノードとプロパティを持つチャンネル群 |
| scene | ルートノードの集合。エンジンのSceneライフサイクルとは別の概念 |

## 現在の構成と設計方針

現在のモデル読み込みは`renderer_vulkan/src/model.rs`のOBJ読み込みと`SourceMesh`が中心です。既存のOBJ経路を維持し、階層・複数primitive・skinを持つモデルを単一のSourceMeshへ無理に平坦化しません。

`kani_volcano_math::Transform`は度数法のEuler角を持っています。glTFの回転にはQuaternionを使うため、アニメーション用ローカル姿勢はQuaternionで保持します。毎フレームEuler角へ変換する方式は避け、既存Transformとの境界を明確にします。静的nodeのmatrixも保持できる設計にし、階層で生じる非一様スケール等を無理にTRSへ再分解しません。

既存の`AssetLoadSystem`はCPU側の準備とGPU登録を分離しています。この流れにモデル全体の読み込みを組み込みます。

### 責務の分担案

| 配置 | 責務 |
| --- | --- |
| `crates/kani_volcano_animation`（新設候補） | ノード姿勢、Skeleton、Skin、AnimationClip、時刻サンプリング、AnimationPlayer、関節行列のCPU計算 |
| engineのモデル読み込み層（配置は段階0で決定） | gltfクレートによる読み込み、CPUモデルデータへの変換、Resourcesへの登録 |
| `crates/engine` | コンポーネント、System、Scene/UpdateContext向けAPI、アセット寿命管理 |
| `crates/renderer_vulkan` | スキニング頂点形式、GPUバッファ、descriptor、パイプライン、シェーダー、draw |
| `crates/kani_volcano_math` | 共有する数学型・変換規約。変更は必要な範囲に限定 |

animationクレートはVulkanやengineに依存させず、CPUだけで検証できるようにします。gltf固有の読み込みと汎用のアニメーション評価も分離し、コードで生成したアニメーションを同じ評価処理に渡せるようにします。独立したモデルアセット用クレートが必要かは、段階0で依存関係を確認して決めます。

共有する不変アセット（メッシュ・クリップ・階層・inverse bind matrix）と、個体ごとの状態（ローカル姿勢・グローバル行列・再生時刻・GPU関節パレット）を分けます。

## 段階0：データ構造と最初の対応範囲

- [ ] 既存のモデル・Material・Resources・Schedulerとの接続箇所を一緒に確認する。
- [ ] CPUモデルデータ、NodeId、Skin、Skeleton、LocalPose、AnimationClipの最小構造を決める。
- [ ] モデル内NodeIdとECS EntityIdの対応方針を決める。初期案はモデル個体内の配列で階層を保持する。
- [ ] 列ベクトル、行列の乗算順、右手系、単位、Quaternionの成分順を文書化する。
- [ ] gltfクレートのバージョンと機能、検証用アセット・ライセンスを選ぶ。

完了条件：依存方向と最小モデル構造を説明でき、最初に表示するアセットと未対応機能が決まっている。

## 段階1：glTF / GLBの静的モデル読み込みと描画

- [ ] gltfクレートでJSON / GLB、外部bin、埋め込みバッファ、画像を読み込む。相対パスはモデルファイル基準で解決する。
- [ ] accessor readerを使い、POSITION・NORMAL・TEXCOORD_0・indicesを読み込む。インデックスなし、属性欠落、stride・正規化・sparse accessorの扱いも確認する。
- [ ] 初期の描画方式はTRIANGLESを対象とし、未対応方式は明示的に報告する。
- [ ] primitiveごとのマテリアルを維持し、base color factorとbase color textureから対応する。UVなし・法線なしの生成／代替／拒否方針を定義する。
- [ ] PNG / JPEG、画像形式変換、sRGB、サンプラーを確認する。alpha mode、double-sided、metallic/roughness等は対応表に記録し、完全なPBR対応と混同しない。
- [ ] nodeのmatrix / TRSと親子階層、複数mesh、同一meshを参照する複数nodeを扱う。
- [ ] default sceneの選択と、default指定がない場合の扱いを定義する。
- [ ] 既存のアセット準備・GPU登録経路に接続する。

完了条件：同内容のgltfとglbを表示でき、親子変換と複数primitiveのマテリアルが保持される。既存OBJ表示も維持する。

## 段階2：Skeletonと姿勢計算

- [ ] animationクレートをworkspaceに追加する。
- [ ] 親子関係、初期ローカル姿勢、個体ごとの現在姿勢を保持する。
- [ ] 親から子の順に`global = parent_global * local`を計算する。
- [ ] jointではない祖先・中間nodeの変換も含める。skin.jointsだけを並べて階層と見なさない。
- [ ] skinごとのjoint一覧、joint番号からNodeIdへの対応、inverse bind matrixを保持する。行列が省略されている場合は単位行列を使う。
- [ ] 範囲外参照や不正な階層をエラーとして扱う。

完了条件：二関節のテストで親の回転が子に伝わる。非jointの中間nodeと、階層順と異なるjoint一覧でも正しく計算できる。

## 段階3：手動姿勢変更とSkinning

- [ ] JOINTS_0 / WEIGHTS_0を読み込み、初期対応は頂点ごとに最大4影響とする。追加セットの扱いは明示し、黙って完全対応としない。
- [ ] joint番号がskin.joints内の番号であることを維持し、参照範囲・ウェイトの正規化・ゼロ和の扱いを検証する。
- [ ] CPUで関節パレットを作り、GPU頂点シェーダーで頂点を変形する。CPUスキニングは小さい数値検証用に限定する。
- [ ] スキニング用頂点レイアウト、パイプライン、関節バッファとdescriptorを追加する。
- [ ] 個体・skinごとのパレットを管理し、GPU使用中のバッファを上書きしない。フレーム単位の領域と上限を決める。
- [ ] 位置だけでなく法線も変形し、非一様スケールへの対応範囲を決める。

初期案はメッシュローカル空間へ戻す方式とします。G_meshとG_jointは同じモデル空間で計算し、Entityの配置変換はその外側で一度だけ適用します。

```text
palette[j] = inverse(G_mesh) * G_joint[j] * inverse_bind[j]
deformed_position = sum(weight[j] * palette[j]) * position
world_position = entity_world * G_mesh * deformed_position
```

この方式ではG_meshが逆行列を持たない場合のエラー方針を決めます。初期node姿勢とbind poseが常に同一とは仮定しません。

完了条件：既知のbind poseを持つ二関節モデルで形状が保たれ、手動回転で期待した頂点が曲がる。モデル全体を移動しても二重変換されない。GPU結果をCPUの期待値と比較する。

## 段階4：AnimationClipと任意時刻の評価

- [ ] キー時刻、対象NodeId、translation / rotation / scale、補間方式を持つチャンネルを実装する。
- [ ] glTF animationを読み込み、ファイル非依存のAnimationClipに変換する。対象はjointに限らない。
- [ ] LINEARを実装する。位置・スケールは線形補間、回転は正規化Quaternionの最短経路の球面線形補間を使う。
- [ ] STEPを続けて対応する。CUBICSPLINEは後続対象とし、未対応の間は明示的にエラーにする。
- [ ] 任意時刻を指定してサンプリングできるようにする。キー前後、1キーのみ、異なるチャンネル時間範囲を定義する。
- [ ] キーのないプロパティは初期ローカル姿勢を使い、前に再生したクリップの値を残さない。
- [ ] コードから同じクリップ構造を組み立てられるようにする。

完了条件：開始・中間・終了時刻のTRSが数値テストと一致する。同じ時刻の評価結果が直前の再生状態に依存しない。

## 段階5：AnimationPlayer

- [ ] クリップ選択、再生、一時停止、再開、停止、seek、ループ、再生速度を実装する。
- [ ] 初期案では停止は時刻0に戻してその姿勢を適用、一時停止は現在姿勢を保持とする。実装時にユーザーと確認する。
- [ ] 大きいdelta time、複数周回、長さ0、非ループ終了時の最終姿勢を扱う。逆再生は後続対象とする。
- [ ] 時刻更新と姿勢サンプリングを分ける。CPU側で再生状態を管理する。

完了条件：読み込んだ歩行アニメーションが自動再生され、ループ・停止・再開・seekが期待通り動く。

## 段階6：ECS・Scene・Systemとの統合

- [ ] ModelInstance、SkeletonPose、AnimationPlayer相当のコンポーネントと、共有アセットIDを定義する。
- [ ] AnimationSystemをSchedulerへ組み込み、既存TimeApi・停止・時間倍率との関係を決める。
- [ ] 基本の実行順を「ゲームロジック → クリップ評価 → ユーザーSystemの姿勢補正 → 階層計算 → 関節パレット生成 → 描画情報抽出 → GPU描画」とする。
- [ ] 手動姿勢モードとクリップ再生モードを区別し、ユーザーSystemの変更が次の評価で意図せず消えないようにする。
- [ ] Sceneのon_enterでモデル配置・再生開始、UpdateSystemからクリップ変更・関節回転を操作できるAPIを設計する。
- [ ] 非同期ロード完了前の再生要求、Entity削除、Scene退出時の参照解放とGPU資源寿命を定義する。
- [ ] RenderItemには必要な描画参照・パレット参照を渡し、AnimationPlayer自体の評価はrendererへ持ち込まない。

完了条件：同じモデルを二体配置し、別時刻・別クリップ・別の手動補正で独立して動く。Scene切替後に不要な資源が残らない。

## 初回完成後に扱う機能

クリップ間のブレンド／クロスフェード、AnimationStateMachine、加算アニメーション、IK、root motion、イベント、リターゲティング、モーフターゲット、CUBICSPLINE、4影響を超えるウェイト、アニメーションLOD、ジョブ並列化は別段階で検討します。圧縮や必須glTF拡張は対応状況を検証し、未対応の必須拡張は読み込み時に報告します。

## 検証と記録

CPUの階層・補間・行列計算には数値テストを用意し、GPUの変形は小さい専用モデルとVulkan validationで確認します。各段階の完了時に、このファイルのチェック欄と以下の記録を更新します。

| 段階 | 状態 | 確認結果・決定事項 |
| --- | --- | --- |
| 0：構造・範囲 | 未着手 | 次にユーザーと取り組む候補 |
| 1：静的モデル | 未着手 | |
| 2：Skeleton | 未着手 | |
| 3：Skinning | 未着手 | |
| 4：AnimationClip | 未着手 | |
| 5：AnimationPlayer | 未着手 | |
| 6：ECS統合 | 未着手 | |

## 参照資料

- [glTF 2.0仕様](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)：形式、node、animation、skinの規約。
- [Khronos glTF Tutorials](https://github.khronos.org/glTF-Tutorials/gltfTutorial/)：静的モデルからアニメーションまでの段階的な例。
- [Khronos Skinning Tutorial](https://github.com/KhronosGroup/glTF-Tutorials/blob/main/gltfTutorial/gltfTutorial_020_Skins.md)：joint一覧・inverse bind matrix・関節パレットの計算。
- [gltfクレート](https://docs.rs/gltf/latest/gltf/)：文書・buffer・imageの読み込みAPI。実装時に採用バージョンのAPIを確認する。

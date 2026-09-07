# Turbo engine architecture

# Turbo Engine TODO
* ~~`App`、`World`、`Renderer`、`Input`、`Time`の責務を分ける~~
* ~~Entityを導入 -> ゲーム内オブジェクトをIDで管理する~~
* ~~Transform、MeshRenderer、Cameraなどのコンポーネント構造を決める~~
* ~~カメラの作成~~
* ~~入力、更新、描画を分けてゲームループを作る~~
* ~~カメラを操作できるように、キーボード入力を作る~~
  - ~~press~~
  - ~~down~~
  - ~~up~~
* ~~2D図形のヘルパー関数~~
* マテリアル
  - ~~Material構造体作成~~
    - ~~color~~
  - ~~renderer_vulkanに渡す~~
    - ~~RenderItemにmaterial_color~~
    - ~~command_buffer更新~~
      - ~~push constantのrangeを変える (pipeline.rs)~~
      - ~~update_secondary_bufferの更新 (command.rs)~~
    - ~~shader更新~~
* ~~Pipeline選択~~
  - 一旦pipelineは１つだけ
  - VulkanDataのpipelineを`Vec<GraphicsPipeline>`にする
  - Material has RenderMode <-> RenderItem has pipeline_key
  - pipeline_key -> GraphicsPipeline
  - vulkan_renderer
    - type.rs : vulkan_data, renderitem
    - pipeline.rs : create_pipeline()
    - commands.rs
    - lib.rs : swapchain
* ~~Image複数作成~~
  - ~~Texture struct と textures: Vec<Texture> を作る~~
  - ~~create_texture_image(path) -> Texture に変える~~
  - ~~shader の set/binding を分ける~~
  - ~~create_descriptor_set_layout を global/material に分割~~
  - ~~pipeline layout に両方渡す~~
  - ~~create_global_descriptor_sets を作る~~
  - ~~create_material_descriptor_sets を作る~~
  - ~~RenderItem に material_index または texture_index を持たせる~~
  - ~~command buffer で global set と material set を bind~~
* Pipeline複数作成
  - ~~DebugLine3D: 線を描画~~
  - ~~Transparent3D: 透明オブジェクトの生成~~
  - ~~Lit3D: ライティング付きの3Dオブジェクトを描画~~
    - ~~vertexにnormalを追加~~
    - ~~VertexLit3Dの作成~~
    - ~~create_source_vertexらへんの修正~~
    - ~~pipeline: VertexLit3Dの作成~~
    - ~~create_lit3d_pipelineの作成 + create_pipelinesに追加~~
    - ~~command_buffer編集~~
    - ~~複数ライトを持たせる~~
      - ~~light descriptor setの作成~~
      - ~~Directional LightとAmbientを1つ~~
      - ~~PointLightの追加 (位置)~~
      - ~~SpotLightの追加 (位置+方向)~~
  - ~~Ui2D: 2DのUIや画像を画面に貼り付ける~~
  - ~~Skybox: 空や背景描画~~
  - ShadowMap: 色を出さずに、depthだけを描画
  - 描画関数を使いやすく抽象化
* ECS化
  - ~~ComponentPoolの作成~~
	- ~~worldにComponentPoolを持たせ、WorldObjectを削除~~
	- ~~system作成~~
		- ~~app.rs: update_camera, prepare_renderer~~
		- ~~world.rs: update~~
		- ~~-> src/system/camera.rs, rotate.rs, rander.rs~~
	- ~~Registry作成~~
		- has ComponentPools
	- ~~App::process_input -> InputSystem / SpawnSystem ~~
	- ~~System を Scheduleにまとめる~~
	- ~~Appのcommand処理をcommand_systemに移動する~~
	- ~~Appが持つmodelsやtexturesをResourceに移動する~~
	- ~~ユーザがsystemを登録できるようなインターフェースの作成~~
	- query3 や optional component query が必要になったら追加
	- ScheduledUpdateSystemを優先度を扱えるように作成
* ~~オブジェクトAPI~~
  - ~~spawn_model~~
  - ~~spawn_cube~~
  - ~~despawn~~
  - ~~get/set transform~~
  - ~~tag/name検索~~
* ~~MeshAsset管理~~
	- ~~現状はEntityがdespawnされてもloadされたmeshは残り続ける~~
	- ~~Entityごとにmeshを持たせる場合はメモリリークになる~~
	- ~~参照カウンタ的なものでdespawn時に自動的にdestroyする~~
	- ~~assetのrelease (Resources.release_mesh)~~
* ~~Component、SystemのAPI準備~~
	- ~~UpdateContext、CommandContext、RenderContextのフィールド pub -> private~~
	- ~~Object共通処理 ObjectApi trait~~
	- ~~Entity EntityApi~~
	- ~~<system>.rsではworldやrendererを触らせない~~
	- ~~systemはContext情報のみでupdateする~~
	- ~~既存のsystemをcontext api経由にする~~
	* EntityApi      spawn / despawn / component / query / name / tag
	- InputApi       mouse_position / window_size / key state
	- ObjectApi      spawn_model / spawn primitive
	- AssetApi       texture / model_asset_id / mesh info
	- RenderCommand	 update_primitive
	- TimeApi        delta_seconds

* ~~シーン管理~~
  - ~~Scene trait~~
  - ~~SceneManager~~
  - ~~current_scene~~
  - ~~change_scene~~
  - ~~sceneごとの初期化/update~~

* ~~ドキュメント~~
* ~~Scene更新API~~
	- ~~今は、Scene更新するたびにAppで`set_current_scene()`~~
	- ~~Sceneからできるようにする~~

* ~~固定フレーム~~

* フォント/UI
cosmic-text + etagere
フォントの読み込み
1. ユーザー指定のTTF/OTFファイルを読み込み・解析
2. Resourcesに登録し、FontAssetIdを返す

テキストの描画準備
3. 文字列・フォント・サイズから、グリフと配置を決定
4. 必要なグリフをラスタライズして画像化
5. グリフ画像をアトラスに詰め込み、キャッシュ
6. 配置とUVから描画用の頂点・インデックスを生成

GPU描画
7. アトラス画像と描画データをrenderer_vulkanへ転送
8. テクスチャを貼った四角形として文字を描画
- `app.load_font(name,path)`
- `assets.font_asset_id(name)`


* 当たり判定
  - BoxCollider
  - SphereCollider
  - intersects
  - query_collisions

* セーブ機能

* 簡易物理
  - Velocity
  - Gravity
  - KinematicBody
  - move_and_slide / move_and_collide


* ShaderApi
	- shader作成
	- pipelineを指定し、shaderの引数を制限

## ユーザ触れられるもの
* App
	* CommandContext
	* UpdateContext
	* RenderContext
* Command
	* UpdateContext
* Component
* EntityId
* Transform
* Material
* PrimitiveShape
* PrimitiveType
* MeshAssetId
* Camera
* Rotator
* Visibility
* Name
* Tag


    

## ログ表示
```rust
log::trace!("trace"); // 詳細情報
log::debug!("debug"); // デバッグに役立つ内部情報
log::info!("info"); // 主要イベント
log::warn!("warn"); // 処理は継続できるが注意が必要
log::error!("error");
```

```
$env:RUST_LOG="debug"
cargo run
```
では
```
debug
info
warn
error
```
が表示される

```rust
#[cfg(debug_assertions)]
log::debug!("camera = {:?}, camera");
```
はreleaseではコンパイルされない

## ECS
```cpp
// エントリポイント
int main() {
	// ECSのレジストリを作成
	Registry registry;

	// 新しいエンティティを作成し、位置と速度のコンポーネントを追加
	Entity player = registry.create();
	registry.addComponent(player, Position{ 0, 0 });
	registry.addComponent(player, Velocity{ 1, 0.5f });

	// 位置と速度を持つエンティティの位置を更新
	MovementSystem(registry);

	// 更新後の位置を表示
	auto& pos = registry.getComponent<Position>(player);
	std::cout << pos.x << ", " << pos.y << "\n";
}
```
### Registry
`Registry`は`Entity`と`Component`を登録・登録  
しかしこのままだと、`addComponent`するたびにメモリを確保し、`forEach`ではIdの全てを走査する。これは効率が悪い。そのため、`SparseSet`という各コンポーネントが登録されているIdと`component`のindexとの対応表を持つようにする。
```cpp
//-------------------------------------------------------------------------
//! @class  Registry
//! @brief  ECS のレジストリクラス。エンティティの管理とコンポーネントの格納を行う。
//-------------------------------------------------------------------------
class Registry {
public:
	//-------------------------------------------------------------------------
	//! @brief 新しいエンティティを作成する。エンティティIDは単純にインクリメントされる。
	//-------------------------------------------------------------------------
	Entity create() {
		return nextEntityId++;
	}

	//-------------------------------------------------------------------------
	//! @brief  エンティティにコンポーネントを追加する。必要に応じてストレージを拡張する。
	//! @tparam T 追加するコンポーネントの型
	//! @param  e 対象のエンティティID
	//! @param  component 追加するコンポーネントのインスタンス
	//-------------------------------------------------------------------------
	template<typename T>
	void addComponent(Entity e, T component) {
		// コンポーネントストレージを取得または作成
		std::vector<std::optional<T>>& storage = getOrCreateStorage<T>();
		// エンティティIDに対応する位置にコンポーネントを配置。必要ならストレージを拡張。
		if (e >= storage.size()) {
			// ストレージをエンティティIDに合わせて拡張
			storage.resize(e + 1);
		}
		// コンポーネントを配置
		storage[e] = component;
	}

	//-------------------------------------------------------------------------
	//! @brief	エンティティが特定のコンポーネントを持っているか確認する。
	//! @tparam T 確認するコンポーネントの型
	//! @param	e [in] 対象のエンティティID
	//! @return エンティティがコンポーネントを持っていれば true、そうでなければ false
	//-------------------------------------------------------------------------
	template<typename T>
	bool hasComponent(Entity e) const {
		// コンポーネントストレージを取得
		std::vector<std::optional<T>>* storage = tryGetStorage<T>();
		// ストレージが存在し、エンティティIDが範囲内で、かつその位置にコンポーネントが存在するか確認
		return storage && e < storage->size() && (*storage)[e].has_value();
	}

	//-------------------------------------------------------------------------
	//! @brief	エンティティの特定のコンポーネントを取得する。存在しない場合は例外を投げる。
	//! @tparam T 取得するコンポーネントの型
	//! @param	e [in] 対象のエンティティID
	//! @return エンティティが持つコンポーネントの参照
	//-------------------------------------------------------------------------
	template<typename T>
	T& getComponent(Entity e) {
		// コンポーネントストレージを取得
		std::vector<std::optional<T>>& storage = getOrCreateStorage<T>();
		// エンティティIDが範囲内で、かつその位置にコンポーネントが存在するか確認
		return *storage[e];
	}

	//-------------------------------------------------------------------------
	//! @brief	エンティティの特定のコンポーネントを取得する。存在しない場合は例外を投げる。
	//! @tparam	T 取得するコンポーネントの型
	//! @return エンティティが持つコンポーネントの参照
	//-------------------------------------------------------------------------
	template<typename T>
	std::vector<std::optional<T>>* tryGetStorage() {
		// コンポーネントストレージを型で検索
		auto it = components.find(typeid(T));
		// ストレージが存在しない場合は nullptr を返す
		if (it == components.end()) return nullptr;
		// ストレージが存在する場合は型をキャストして返す
		return &std::any_cast<std::vector<std::optional<T>>&>(it->second);
	}

	//-------------------------------------------------------------------------
	//! @brief	エンティティの特定のコンポーネントを取得する。存在しない場合は例外を投げる。
	//! @tparam T 取得するコンポーネントの型
	//! @return エンティティが持つコンポーネントの参照
	//-------------------------------------------------------------------------
	template<typename T>
	const std::vector<std::optional<T>>* tryGetStorage() const {
		// コンポーネントストレージを型で検索
		auto it = components.find(typeid(T));
		// ストレージが存在しない場合は nullptr を返す
		if (it == components.end()) return nullptr;
		// ストレージが存在する場合は型をキャストして返す
		return &std::any_cast<const std::vector<std::optional<T>>&>(it->second);
	}

	//-------------------------------------------------------------------------
	//! @brief	エンティティの特定のコンポーネントストレージを取得する。存在しない場合は新たに作成する。
	//! @tparam T 取得または作成するコンポーネントの型
	//! @return エンティティが持つコンポーネントの参照
	//-------------------------------------------------------------------------
	template<typename T>
	std::vector<std::optional<T>>& getOrCreateStorage() {
		// コンポーネントストレージを取得
		std::vector<std::optional<T>>* ptr = tryGetStorage<T>();
		// ストレージが存在しない場合は新たに作成してマップに追加
		if (!ptr) {
			// 新しいストレージを作成してマップに追加
			components[typeid(T)] = std::vector<std::optional<T>>();
			// 作成したストレージを再度取得
			ptr = tryGetStorage<T>();
		}
		// ストレージを返す
		return *ptr;
	}

	//-------------------------------------------------------------------------
	//! @brief  指定したコンポーネントをすべて持つエンティティに対して処理を行う
	//! @tparam Components 対象となるコンポーネント型
	//! @tparam Func       実行する関数（ラムダ）
	//-------------------------------------------------------------------------
	template<typename... Components, typename Func>
	void forEach(Func&& func) {
		// 1. 各コンポーネントのストレージをまとめて取得
		auto storages = std::tuple{ tryGetStorage<Components>()... };

		// 2. どれか一つでもストレージが存在しなければ終了
		bool allStoragesExist = std::apply([](auto... ptrs) {
			return ((ptrs != nullptr) && ...);
			}, storages);
		if (!allStoragesExist) return;

		// 3. 最小サイズを求める
		size_t count = std::apply([](auto... ptrs) {
			return std::min({ ptrs->size()... });
			}, storages);

		// 4. 各エンティティを走査
		for (size_t e = 0; e < count; ++e) {

			// 4-1. 全コンポーネントが存在するかチェック
			bool allExist = std::apply([&](auto... ptrs) {
				return (((*ptrs)[e].has_value()) && ...);
				}, storages);
			if (!allExist) continue;

			// 4-2. 参照を取り出して関数に渡す
			std::apply([&](auto... ptrs) {
				func((*(*ptrs)[e])...);
				}, storages);
		}
	}


private:
	Entity nextEntityId = 0;									// 次に割り当てるエンティティID
	std::unordered_map<std::type_index, std::any> components;	// コンポーネントストレージを型ごとに管理するマップ	
};
```

### Entity
ただのID
```cpp
Entity player = registry.create();
registry.addComponent(player, Position{ 0, 0 });
registry.addComponent(player, Velocity{ 1, 0.5f });
```

### Component
Entityに性質や状態を与えるためのデータのかたまり

### System
Componentの振る舞い
```cpp
//-------------------------------------------------------------------------
//! @brief 位置と速度を持つエンティティの位置を更新するシステム
//! @param registry ECSのレジストリ
//! @note  位置と速度の両方を持つエンティティのみが対象となる
//-------------------------------------------------------------------------
void MovementSystem(Registry& registry) {
	// 位置と速度を持つエンティティに対して、位置を速度分だけ更新する処理を行う
	registry.forEach<Position, Velocity>([](Position& p, Velocity& v) {
		p.x += v.vx;	// 位置を速度分だけ更新
		p.y += v.vy;	// 位置を速度分だけ更新
		});
}
```



## コアシステム
* ~~アサーション~~
* メモリ管理
* 数学ライブラリ
* ~~Debugビルド用の設定~~
* ~~Releaseビルド用の設定~~
* 独自のアルゴリズムやデータ構造
  - Handle: オブジェクト識別子
  ```rust
  pub struct Handle<T> {
    index: u32,
    generation: u32,
    _marker: std::marker::PhantomData<T>,
  }
  ```

## グラフィックス
* ~~Transformを作成し、`RenderObject`が個別に動けるようにする~~
* マテリアル
* ~~カメラの作成~~
  - ~~平行移動~~
  - ~~前後移動~~
  - ~~方向変更 (Mouse)~~
* pipeline複数生成
* シェーダーを自由に選択できるようにする
* 2d図形のヘルパー関数
  - ~~三角形~~
  - ~~四角形~~
  - ~~直方体~~
  - ~~円~~
  - ~~ポリゴン~~
  - 平面
  - ~~球~~
  - 線分 (pipeline後)
* shaderのコンパイル手順の自動化
* フォント
* ビューポートと仮想スクリーンによるマルチディスプレイ


## ゲームループとアプリ構造
* ~~`App`、`World`、`Renderer`、`Input`、`Time`の責務を分ける~~
* ウィンドウイベント処理をアプリ本体から切り出す
* ~~Entityを導入し、ゲーム内オブジェクトをIDで管理する~~
* ~~Transform、MeshRenderer、Cameraなどのコンポーネント構造を検討する~~
* シーン内のオブジェクト追加、削除、検索を安全に行えるようにする
* 固定更新と可変更新の扱いを決める
* ポーズ、ステップ実行、リセットを実装する
* FPS、フレーム時間、描画オブジェクト数を表示またはログ出力する
* 設定ファイルからウィンドウサイズ、VSync、MSAA、アセットパスを読み込む

## アセット管理

* アセットディレクトリ構成を決める
* モデル、テクスチャ、シェーダーをアセットIDで参照できるようにする
* 同じアセットを二重ロードしないキャッシュを作る
* 読み込み失敗時に代替アセットや分かりやすいエラーを出す
* mipmap生成の有無をアセット単位で選べるようにする
* sampler設定をテクスチャごとに持てるようにする

## ビジュアルエフェクト
* パーティクルシステム
* デカールシステム
* ライトマップと環境マップ
* 動的シャドウ

## 描画品質
* PhongまたはBlinn-Phongの基本ライティングを実装する
* directional light、point light、spot lightを扱えるようにする
* 法線を使った陰影計算を追加する
* ガンマ補正とsRGBテクスチャの扱いを整理する
* アルファブレンドを実装し、透明オブジェクトを描画できるようにする
* 不透明パスと透明パスを分ける
* face culling、front face、depth test、depth writeの設定を整理する
* ワイヤーフレーム表示や法線表示などのデバッグ描画を追加する
* skyboxまたは背景描画を追加する
* シャドウマップを実装する
* bloom、tone mapping、color gradingなどのポストプロセスを検討する
* render passを複数パス構成に拡張できるようにする


## 入力と操作

* キーボード入力を押下、押している間、離した瞬間で扱えるようにする
* マウス移動、クリック、ホイール入力を扱えるようにする
* 入力マッピングを作り、キー設定を変更できるようにする
* カメラ操作、オブジェクト選択、デバッグ操作を入力システム経由にする

## 物理とゲーム機能
* Open Dynamics Engineのようなエンジンを探す


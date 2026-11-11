# KaniVolcanoEngine：動的テキスト更新時のフレーム停止調査

記録日：2026-10-29。対象ゲーム：Planet（Rust）。GPU：Intel Iris Plus。
調査対象はCargo.lockのKaniVolcanoコミット `b269cd01fc2882f3d8773bc3997219e624d65ac1`。
以下のエンジン内パスはリポジトリの `engine/` からの相対パス。

## 結論と確度

戦闘中に60FPS付近から瞬間的に低下する。タイトル・報酬画面、スキル粒子がない場面でも発生する。
動的テキスト変更とエンジン更新末尾の長時間処理が強く相関している。
コード上も、文字列変更時のmesh再生成・GPU転送・旧mesh破棄に同期的な待機経路がある。

ただし、現時点の計測は文字処理だけを直接測っていない。
レイアウト、glyph生成、atlas転送、mesh転送、旧mesh破棄のどれが何msを占めるかは未確定。
「毎回初めての文字だから遅い」「停止の全てがテキスト由来」「GPU描画負荷は無関係」とは断定しない。

## 実機ログの根拠

ユーザー提供の同一セッションから抜粋。seed：17552645196586171157、451 destinations。
開発側の再実機計測ではなく、ユーザーの計測結果。

| 場面・変更内容 | update | before_tail | engine_tail | text_changes |
|---|---:|---:|---:|---:|
| 開始報酬・説明・カード | 127.3ms | 3.4ms | 123.9ms | 5 |
| AMMO 17 / 18 | 51.8ms | 4.3ms | 47.5ms | 1 |
| AMMO 10 / 18 | 50.9ms | 5.9ms | 45.0ms | 1 |
| AMMO 3 / 18 | 50.6ms | 6.1ms | 44.6ms | 1 |
| Gravity Pulse CD 8s / Black Hole ACTIVE 1s | 58.4ms | 5.3ms | 53.1ms | 2 |
| AMMO 14 / 18 | 49.4ms | 6.9ms | 42.5ms | 1 |
| DAMAGE 6.0 / HP 78 / 101 | 55.4ms | 4.9ms | 50.5ms | 2 |
| 文字変更なしの一例 | 10.9ms | 9.1ms | 1.8ms | 0 |

文字変更なしでbefore_tailが約12～21msになるフレームもある。
前フレームの停止で固定更新の追いつき処理が増えた可能性はあるが、固定tick数の計測で確認が必要。
セッション全体の平均値には起動・画面遷移等が含まれるため、戦闘中のFPS保証には使わない。

## 現在のキャッシュの挙動

### glyph画像

- `crates/kani_volcano_text/src/font.rs`：SwashCacheを使用。
- `crates/kani_volcano_text/src/atlas.rs`：登録済みglyphの領域を再利用。既存キーではページをdirtyにしないテストもある。
- glyphのキー条件（フォント・サイズ等）が一致すれば、数字を表示するたびに画像を作り直すわけではない。

### 文字列mesh

- `crates/engine/src/system/text_render_system.rs`
- キャッシュは `HashMap<EntityId, CachedText>`。
- TextLayoutの比較項目はcontent / font / font_size / line_height。
- 同じEntityの内容が変わると `need_rebuild` がtrueとなり、`create_text_mesh` を呼ぶ。
- `store_mesh` は以前のmeshを破棄して現在のmeshへ置き換える。
- 過去に表示した全文字列のmeshを保持する方式ではない。
- 例：同じ弾数ラベルが18→17→16→…→18→17と変わると、二度目の17も文字列meshの再生成対象。
- 色・Transform・Visibilityだけの変更はTextLayoutの変更ではない。

## 調査対象の処理経路

1. `crates/engine/src/app.rs` の `App::update`
   - 固定更新、フレーム更新、資産・音声処理に加えて `run_render_stage` を含む。
   - したがってログのupdateは純粋なゲームロジック時間ではない。
2. `TextRenderSystem::update`
   - layoutが変わった可視Textについてmesh再生成。
3. `crates/renderer_vulkan/src/lib.rs` の `create_text_mesh`
   - `layout_text` → `build_text_mesh` → `upload_text_atlas` → `upload_text_mesh`。
   - atlasはdirty pageの転送方式。呼び出されるだけで必ず全ページを転送するわけではない。
4. `upload_text_mesh`
   - batchごとに `load_mesh_from_data` を呼び、新規meshを生成。
5. `crates/renderer_vulkan/src/buffer.rs` の `end_single_time_commands`
   - `queue_submit` の直後に `queue_wait_idle`。
   - mesh転送等の経路で待機の頻度と所要時間を計測したい。
6. `store_mesh` → `destroy_text_mesh` → `destroy_mesh`
   - `destroy_mesh` は `device_wait_idle` 後に旧meshを解放する。
   - 小さな文字列更新でもデバイス全体の完了待ちを起こし得る。

追加確認点：`load_mesh_from_data` はmesh管理配列へpushし、破棄時はslotをNoneにしている。
長時間の文字更新で配列や関連管理コストが増えないかも確認する。GPUメモリリークと断定するものではない。

## 改善案の優先順位

1. **処理段階別の計測**
   - layout/glyph rasterization/atlas upload/mesh allocation・upload/old mesh destroyの時間と回数。
   - queue_wait_idle/device_wait_idleの回数・待機時間、文字mesh再生成数、atlas dirty page数。
   - CPU時間とGPU実行時間を区別し、可能ならGPU timestampも使用。
2. **動的文字meshのバッファ再利用**
   - 毎回の新規確保・破棄を避け、容量内なら既存領域を更新する設計。
   - in-flightのGPU読み取りと競合しないよう、フレーム別バッファやリングバッファ等を検討。
3. **旧リソースの遅延破棄**
   - device_wait_idleを文字更新ごとに呼ばず、使用フレームのfence完了後に回収。
   - 待機呼び出しを単に削除するとuse-after-free等になるため、寿命管理とセットで変更する。
4. **転送のまとめ処理**
   - 同一フレームの文字・atlas更新をまとめて転送し、更新ごとのqueue_wait_idleを避ける。
   - staging領域の再利用・寿命管理、正しい同期を維持する。
5. **事前準備と上限付きキャッシュ**
   - 数字・記号など実際のフォント／サイズで事前にglyphを準備し、初回負荷をロードへ移す。
   - 弾数等の限定された文字列はmesh/layoutキャッシュも選択肢。ただし無制限保持は避ける。
   - 事前glyph準備だけでは、毎回のmesh転送・旧mesh破棄の同期問題は解消しない。

## ゲーム側で行った回避策

- FPSの可変文字列を固定矩形の7セグメントへ変更。ユーザーから「だいぶ良くなった」と報告あり。
- リロードの文字バーを矩形目盛りへ変更。18発分をVisibilityで段階表示。
- スキル／アクションのCD・ACTIVEは小数秒から整数秒へ変更。READYは即時切り替え。
- HPバー等はTransformで更新。弾数・HP数値・命中数値・CD文字はまだText更新が残る。
- 粒子非発動中の更新も省略したが、粒子なしでも停止が残った。

UI情報をさらに削るより、動的テキストの更新経路を改善したい。
エンジン側で解消すれば、必要に応じて小数秒等の表示を再検討できる。

## 再現・検証方法

PowerShell、ゲームのワークスペースで実行：

```powershell
$env:PLANET_FRAME_TRACE='1'
cargo run --release --locked
```

- Titleで待機 → 初期報酬を選択 → 静止したまま射撃して弾数を減らす → リロード → Q/Zスキル → 被弾 → Pause。
- 改善前後は同じseed・解像度・操作条件を揃える。
- 最小再現として、Textを1個だけ配置して「AMMO 18 / 18」「AMMO 17 / 18」を繰り返すテストも有効。
- 初回表示と、同じ数字・文字列を何周も表示した後を分けて計測する。
- 文字変更なし、Transform/色のみ変更、文字列変更、新規glyph追加の4条件を比較する。
- 平均FPSだけでなくフレーム時間p95/p99、最大時間、25ms超の件数も確認する。
- 長時間更新・リサイズ・報酬画面・シーン再入場・終了時のリソース回収も確認する。
- Vulkan validation error、表示欠け、古い文字残り、メモリ増加がないことを確認する。

現在のゲーム計測：`src/system/frame_trace_system.rs`、`src/platform.rs`。

- interval：フレーム開始間隔。前フレームの停止を含む。
- update：App::update全体。
- before_tail：App::update開始からゲーム側の最後のupdate System終了まで。前段の資産処理等も含む。
- engine_tail：そのSystem終了からApp::update終了まで。音声・資産処理・描画準備等を含む。
- render_wait：App::renderのCPU側所要時間。GPU単体の実行時間ではない。
- text_changes：可視Textのlayout変更をゲーム側で比較した推定件数。GPU実測の再生成数ではない。
- スパイク出力は最大毎秒1回のため、全遅延フレームを網羅しない。
- 計測SystemはPLANET_FRAME_TRACE指定時のみ登録する。

解除：`Remove-Item Env:PLANET_FRAME_TRACE`

## 別件のログ

`EOSOverlayVkLayer-Win64.json` が開けないというEpic Games OverlayのVulkan loaderエラーも起動時に出ている。
今回のテキスト変更と連動した停止との因果関係は確認できていない。今回のゲーム変更ではOSやOverlay登録を変更していない。

## 現時点の到達点

ゲーム側回避策・計測追加はコンパイルチェック済み。
エンジン側の改善は未実施で、チェックアウトされたCargoキャッシュも変更していない。
ユーザーがエンジンを改善後、ゲームの依存リビジョンを更新して同条件で再検証する。


## シーン終了時のEntity削除順序（2026-10-30追記）

6d6b590の`Scene::on_exit_scene`は`despawn_scene_owned_entities()`を先に実行し、その後`on_exit()`を呼ぶ。
Planetでは`on_exit()`でBgmStateなどのComponentから再生IDを取得して音声を停止するため、既にEntityが削除されており停止対象を取得できなかった。
音声リソースはシーンをまたいで残るため、Tab → T → Enterで旧ループBGMが残り、新しいBGMと重なり続ける。周回引き継ぎ用のRunProgress取得も同じ順序の影響を受ける。

ゲーム側は`RunScene::on_exit_scene`をoverrideし、`on_exit()`で音声停止命令と周回引き継ぎを実行してからEntityを削除する順序に変更した。
`Scheduler::reset_scene_runtime()`は音声コマンドキューを消去しないため、ここで発行した停止命令は保持される。
エンジン側のソースは変更していない。エンジン標準処理も終了コールバックを削除前に実行する設計を検討できる。
実機でのタイトル/Continue反復後の聴感・FPS・再生数は別途確認対象。

### 2026-10-30 ゲーム側の軽減策
Icon HUDの内容依存フォントサイズを固定枠サイズへ変更。TextWarmupSystemでHUD使用サイズのASCIIを複数の接頭辞付きでシーン開始時・ウィンドウサイズ変更時に準備し、準備Entityは翌更新で削除する。新規glyph配置やカード等の別サイズは対象外となり得るため、atlas同期停止を完全に解消する変更ではない。実機FPS改善は未計測。

## フォントアトラス転送の最適化依頼（2026-10-30）

### 結論と対象

動的テキストのメッシュバッファ再利用は機能している。残る主な調査・改善対象は、新規glyph（描画用の文字画像）をアトラスへ転送する処理である。
Planetの制作を止める必須修正ではないが、他のゲームでも文字更新時の停止を抑えるために、エンジン共通機能として改善する価値がある。
この節は追加調査後の報告であり、上記の旧版に対する「エンジン側の改善は未実施」という記述とは区別する。今回、ゲーム側からエンジンのソースは変更していない。

### 環境と計測証拠

- Windows / Intel Iris Plus Graphics / Planet releaseビルド。
- エンジン参照リビジョン：6d6b590eadfffbe45b15eb3bdfdebac517f69afd。
- PLANET_FRAME_TRACEとKANI_TEXT_TRACEを有効化したユーザー実機ログ。

代表例：

```text
[kani_text] layout=0.091ms glyph_mesh=0.048ms atlas=48.386ms mesh_write=0.010ms batches=1
Frame spike: mode=Playing interval=15.6ms update=55.9ms render_wait=1.2ms visible_meshes=58 skill_parts=0
  Update breakdown: before_tail=0.7ms engine_tail=55.3ms text_changes=1 ["AMMO  17 / 18   [RMB]"]
```

別の更新でもatlasに37～48ms程度かかる例が繰り返された。一方、layout / glyph_meshは多くの例で1ms未満、mesh_writeは概ね0.005～0.3ms程度で、atlasが0.001ms程度で済む更新もある。これらはログの観測値であり、統計的なp95/p99ではない。

- 今回の文字更新スパイクでは、CPUレイアウトや文字メッシュ書き込みよりatlas区間が支配的。
- atlasのCPU所要時間には同期の待ち時間も含まれる。GPUのコピー処理そのものが約48msかかるという意味ではない。
- engine_tailは音声・資産処理・描画準備も含むため、全時間をテキスト処理と断定しない。
- ログ出力にも負荷がある。最終的なFPS・フレーム時間比較では無効化する。

### ゲーム側で実施した軽減策と結果

1. アイコン横のスキル・回復・ジャンプデバイス表示とCD数字を、表示内容によってサイズが変わらない固定枠方式に変更した。ウィンドウサイズへの追従は維持。HP・弾数は従来からウィンドウ基準のサイズである。
2. TextWarmupSystemで、HUDに使うサイズのASCII文字を複数の接頭辞付きでシーン開始時・ウィンドウサイズ変更時に画面外で1フレーム描画する。準備用Entityは次の更新で削除する。
3. カードやメニューの長文フィットは維持した。

ユーザーからrelease版では引っかかりが軽減したとの報告あり。改善率・p95/p99・メモリ増分は未計測。通常ビルドでは40～60FPSの変動が残るとの報告があるが、その原因は追加ログなしでは断定できない。

事前準備は停止を開始時へ移す回避策であり、新しい文字・サイズ・サブピクセル配置によるアトラス更新を完全には防がない。起動処理とアトラス使用量も増える。

### エンジン側の推奨優先順位

| 優先 | 対応 | 目的 |
|---|---|---|
| 1 | atlas更新区間の計測を細分化する | メモリ確保、stagingへの書き込み、コマンド記録、送信、待機のどこに時間を使うか確認する |
| 2 | アトラス転送時の全体待機を避け、必要な同期へ置き換える | 戦闘中に新しい文字が出たときの大きな停止を抑える |
| 3 | 同一フレームの更新をまとめ、dirty領域だけ転送する | 複数Text更新時の送信回数・転送量を抑える。現行実装で対応済みの範囲は調査して重複実装を避ける |
| 4 | フォント・サイズ・文字集合を指定できる事前準備APIを用意する | 画面外Entityを使うゲーム側回避策を不要にし、ロード画面で計画的に準備できるようにする |

文字メッシュのHOST_VISIBLEバッファ再利用は維持する。今回の優先対象は全文字列キャッシュの追加ではない。

### 同期・寿命管理の要件

単にqueue_wait_idle等の待機を削除するだけでは不十分。アトラスを参照する描画と転送の順序、画像レイアウト遷移、読み書きの依存関係を保証すること。

- 同じアトラス画像を使用中のフレームとの競合を防ぐ。
- staging領域・転送コマンド等は、GPU使用完了前に上書き・解放しない。
- 新しいページの確保、ページ拡張や差し替えがある場合のdescriptor更新も寿命管理する。
- 専用転送queueを使う設計ならqueue間の同期・所有権移譲も扱う。専用queueの導入自体を必須とはしない。
- 非同期化で未完了リソースが無制限に蓄積しないよう、使用量と回収を管理する。
- 初回文字が欠ける、古い文字になる、表示が不定期に遅れるといった退行を避ける。

### 検証と完了判断

同一seed・解像度・操作条件のrelease版で、修正前後を比較する。

1. 既存glyphのみの弾数・HP・CD更新。
2. 未使用の数字・記号・英字、日本語、異なるフォントサイズを順に追加する初回表示。
3. 複数Textの同時更新、空文字、長文、報酬カード、惑星マップ。
4. リサイズ、シーン遷移、長時間更新、終了時の回収。
5. Intel Iris Plusに加え、可能なら独立GPUでも比較。

段階別計測は上記ボトルネックの切り分けに使用し、最終比較ではログを無効にして通常フレーム時間・p95/p99・最大時間・25ms超の件数・メモリ使用量を記録する。Vulkan検証エラー、表示欠け、継続的なメモリ増加がないことも確認する。
60FPSの目安は1フレーム約16.7ms。ただし文字処理だけでこの時間を使用してよいという意味ではなく、ゲーム全体のフレーム予算に収まるかで判断する。

### 他のゲームへの有用性

HP・弾数・タイマー・スコア・会話・インベントリなど、動的テキストを使うゲームに共通する改善となる。特に日本語会話、ユーザー入力、可変フォントサイズなど、使用glyphを事前に限定しにくいUIで価値がある。
静的テキストのみの画面や、既存glyphだけで運用できるゲームでは恩恵が小さい場合もある。改善率はゲームと実機に依存するため、Planetの観測値を他タイトルの性能保証には使わない。

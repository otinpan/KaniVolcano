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

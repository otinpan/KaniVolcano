# 動的テキストのバッファ再利用

文字列変更時のメッシュを、文字専用の HOST_VISIBLE / HOST_COHERENT バッファへ書き込む。
通常のメッシュ転送で行っていた、頂点・インデックスごとの queue_wait_idle を通らない。
文字列レイアウトとCPUメッシュは引き続き再生成する。全文字列のキャッシュではない。

## 寿命とメモリ

- 描画の送信に成功した時点で、そのメッシュを使用するフレームのビットを記録する。
- フレームの既存fenceを待った後にビットを解除する。全使用フレームが完了するまで、旧メッシュを書き換えたり解放したりしない。
- 使用完了したバッファから、必要容量を満たす最小のものを再利用する。容量不足なら新規確保する。
- 未送信のメッシュはすぐ再利用できる。更新だけを繰り返しても、完了しないフレームを待って蓄積しない。
- 未使用キャッシュはバッファ容量合計16MiB・1024メッシュまで。実メモリにはVulkanのアラインメントや管理領域が加わる。表示中・GPU使用中の領域は別枠。
- 上限を超えた未使用バッファは解放し、空いた管理スロットも再利用する。
- 終了時は従来どおりデバイス完了を待ち、管理配列内の全バッファを回収する。

新しいglyphのアトラス転送には従来の同期処理が残る。初回表示の停止まで解消したという意味ではない。
通常メッシュの転送方式は維持する。HOST_VISIBLEメモリからの描画は、特に独立GPUでは実ゲームで性能比較が必要。

## 計測

```powershell
$env:KANI_TEXT_TRACE='1'
# このエンジンを参照するゲームを起動
Remove-Item Env:KANI_TEXT_TRACE
```

有効時のみstderrへ layout / glyph_mesh / atlas / mesh_write / retire のCPU所要時間を出す。
glyph_meshはglyph処理とCPUメッシュ生成の合計。GPU実行時間ではない。
出力自体が実行時間へ影響するため、最終FPS比較では無効にする。

## 検証

```powershell
cargo test -p renderer_vulkan -p kani-volcano-text -p kani-volcano-engine --lib --locked
cargo check --workspace --all-targets --locked

# Windows / Vulkan検証レイヤーが必要。既定フォントはWindows Arial。
# 別フォントなら $env:KANI_TEST_FONT='フォントの絶対パス'
cargo test -p renderer_vulkan text_gpu_tests --locked -- --ignored --nocapture

# テスト内だけで旧転送経路を再現する比較用モード
$env:KANI_TEST_LEGACY_TEXT='1'
try {
    cargo test -p renderer_vulkan text_gpu_tests --locked -- --ignored --nocapture
} finally {
    Remove-Item Env:KANI_TEST_LEGACY_TEXT
}
```

実機テストは非表示の320×240ウィンドウで600フレーム描画し、最初の120フレームを時間集計から除外する。
数字の更新、空文字列、長い文字列、UI/3Dテキストの切り替え、swapchain再生成を含む。
修正後の経路では、追加で未送信更新1000回、キャッシュ上限超の解放、管理スロット再利用を検証する。
Vulkan検証エラーを検出すると失敗する。画面の見た目の検査やGPU timestamp計測は含まない。

2026-10-29、Intel Iris Plus Graphics、debug検証レイヤー有効、段階別ログ無効での一例：

| 文字更新のCPU時間 | 旧経路 | 再利用経路 |
|---|---:|---:|
| p95 | 17.281ms | 0.240ms |
| p99 | 17.794ms | 0.776ms |
| 最大 | 19.765ms | 1.008ms |
| 600フレーム終了時の管理スロット数 | 590 | 3 |

両経路ともVulkan検証エラーなし。単体テスト123件成功。workspace全targetのコンパイル確認成功（既存warningあり）。
この比較は文字更新処理のCPU時間であり、Planet全体のフレーム時間やFPS改善率ではない。
ゲームの依存先をこの修正へ切り替え、同一seed・解像度・操作で通常フレーム時間、p95/p99、メモリ、表示の正しさを別途比較する。

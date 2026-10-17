# ネイティブGPUマップパネル

2026-10-17。通信マップの「大規模マップを開く」から、Tauriメインウインドウ内の専用領域へ直接Metal描画する。通常のアプリ／接続先ペインは併用できる。

## 境界と表示

- 標準ビルドは`gpu`機能を含む。アダプター・サーフェス・メッシュを作るのは明示的に開いた後だけ。`--no-default-features`でもビルドでき、非対応環境／ブラウザでは開く操作を無効化する。コンパイル上の利用可否は実アダプターの動作確認とは区別する。
- macOSのメインスレッド上にRustの単一Controllerを置き、NSView・CAMetalLayer・wgpu Surface／Rendererを所有する。ウインドウやWebViewはDocumentや描画データを所有しない。描画元は既存のRust共有Sessionの接続と選択IDだけで、履歴・診断・Syslog本文はコピーしない。
- JavaScript→Rustは開く／閉じる、表示矩形・クリップ矩形・viewport寸法、lease／sequenceだけ。戻り値は描画受付状態・件数・省略有無。経路配列、頂点、テクスチャ、フレームやピクセルのIPC／CPU readback APIは追加しない。
- 全接続の概要であり、通常ペインの検索・ループバック非表示とは独立する。左はアプリ、右はIP・ポート。安定ソートし、最大10,000接続、選択接続を優先して含め、最後に青で描画する。その他は灰色。線は観測の表示であり、許可／遮断の適用証拠ではない。名前・詳細・ルール操作には通常ペインを使う。
- 範囲、選択色、件数上限と省略を3言語の情報アイコン／件数表示で示す。通常表示を残し、サーフェスの作成・描画エラーではネイティブパネルを解放して通常表示へ戻す。

## ネイティブ接続

`gpu_panel`がTauriコマンドを受け、`gpu_panel_macos`のメインスレッド処理に渡す。署名・管理者認証・Network ExtensionやOS設定変更は不要。

1. 開くたびにRustが新しいleaseを発行する。初期状態は閉じており、開く前の同期は拒否する。前のビューを解放し、古いlease／逆順sequence／終了済み要求で再作成できないようにする。main以外のウインドウからの操作は拒否する。
2. DOMの表示領域をResizeObserver・scroll・resize・focus・visibility・共有revisionと1秒の状態再確認により同期する。requestAnimationFrameで合流し、IPCは1件ずつ送り、待機中の変更は最新の位置へまとめる。
3. viewport、スクロールコンテナ、最上部の保護状態表示でクリップする。Rustでも有限値・範囲・クリップの包含を検証する。NSViewのクリップコンテナ内に元の大きさの描画ビューを配置し、部分スクロールで図を縮めない。
4. TauriのNSWindow contentViewに、マウス／ホイールのhitTestを通過させるネイティブビューを重ねる。通常のWebView操作を妨げず、クリップ層が描画領域からのはみ出しを抑える。
5. CSS viewportとAppKit座標の比を計算し、backing scaleで描画サイズを決める。物理寸法は各辺4096pxまで。サイズが変わればSurfaceを再設定する。ゼロ・完全な領域外・非表示・最小化では描画しない。
6. Surfaceに対応するMetalアダプターとsRGB形式を選び、取得したdrawableへ直接描画してpresentする。表示矩形だけの変更は既存フレームを再利用する。経路とサイズが同じならGPU頂点バッファも再利用する。
7. Surface Lost／Outdatedは再設定して次回同期を待つ。Timeoutも次回同期へ回す。その他の取得失敗はエラーにする。presentは通常の非同期提出であり、実表示や保護状態を証明するものではない。
8. 閉じる／画面切替でleaseを終了し、メインウインドウのresize／閉じる操作では一旦隠す。Destroyedではleaseを終了して解放する。Surfaceを保持している間はCALayerを保持し、ビューを取り外してからSurface→Renderer→CALayer／NSViewを破棄する。AppKit資源をworker threadから操作しない。

## 開発検証

```sh
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --offline
cargo run --manifest-path src-tauri/Cargo.toml --example native_panel_check --features gpu-qa --offline
npm test
npm run build
```

`gpu-qa`は明示指定する統合確認用機能で、標準ビルドには含めない。専用Tauri WebViewウインドウと一時文書だけを使い、観測・更新・拡張機能・ルーター・実ユーザー文書に触れない。実Surfaceへの10,000／10,001接続、GPU完了待ち、クリッピング、サイズ変更、非表示、終了後の遅延拒否、再作成、解放を検査する。描画は読取り観測の図であり、ネットワークを生成しない。

🟢 [Done] 開発Mac上の実Metal Surface／Tauri WebViewとの接続。純粋なシーン／矩形／leaseテスト、GPUなしビルド、3言語UI、IPCをモックしたブラウザでの開く→位置同期→閉じる／画面切替→失敗時の復帰。ブラウザのスクリーンショットにはGPUピクセルは含まれないため、Metalの見た目の実証には使わない。

⭕️ [Pending] 署名済み配布アプリでの実UIとMetalの重なり・ホイール操作・保護状態バーとの実視認、Retina／HiDPI実表示と複数ディスプレイのbacking scale切替、全画面／Space／スリープ復帰、長時間の負荷・メモリ・電力、GPU／device loss、各macOS／Intel環境。実機ハーネスで確認した範囲と、これらの未確認項目を区別する。

🔴 [Later] GPU内のアプリ名／接続先ラベル、ズーム／ヒットテスト、専用GPUパネル自体のフローティング、検索と同一条件の描画。既存詳細パネルのドッキング／フローティングは維持する。

# ネイティブGPUマップパネル

2026-10-17作成、2026-10-25条件追従を追加。通信マップの「大規模マップを開く」から、Tauriメインウインドウ内の専用領域へ直接Metal描画する。通常のアプリ／接続先ペインは併用できる。

## 境界と表示

- 標準ビルドは`gpu`機能を含む。アダプター・サーフェス・メッシュを作るのは明示的に開いた後だけ。`--no-default-features`でもビルドでき、非対応環境／ブラウザでは開く操作を無効化する。コンパイル上の利用可否は実アダプターの動作確認とは区別する。
- macOSのメインスレッド上にRustの単一Controllerを置き、NSView・CAMetalLayer・wgpu Surface／Rendererを所有する。ウインドウやWebViewはDocumentや描画データを所有しない。描画元は既存のRust共有Sessionの接続・ルール意図・選択IDで、履歴・診断・Syslog本文はコピーしない。
- JavaScript→Rustは開く／閉じる、表示矩形・クリップ矩形・viewport寸法、lease／sequenceと検索語・意図・ループバック表示条件・期待する実測／デモの区分だけ。戻り値は描画受付状態・描画／条件一致／全接続の件数・省略有無。経路配列、頂点、テクスチャ、フレームやピクセルのIPC／CPU readback APIは追加しない。
- 通常ペインと同じ検索・ルール意図・ループバック非表示条件でRustが現在の接続を投影する。左はアプリ、右はIP・ポート。安定ソートし、最大10,000接続、条件内の選択接続を優先して含め、最後に青で描画する。その他は灰色。線は観測の表示であり、許可／遮断の適用証拠ではない。名前・詳細・ルール操作には通常ペインを使う。
- 範囲、選択色、件数上限と省略を3言語の情報アイコン／件数表示で示す。通常表示を残し、サーフェスの作成・描画エラーではネイティブパネルを解放して通常表示へ戻す。

## ネイティブ接続

`gpu_panel`がTauriコマンドを受け、`gpu_panel_macos`のメインスレッド処理に渡す。署名・管理者認証・Network ExtensionやOS設定変更は不要。

1. 開くたびにRustが新しいleaseを発行する。初期状態は閉じており、開く前の同期は拒否する。前のビューを解放し、古いlease／逆順sequence／終了済み要求で再作成できないようにする。Rust Sessionが指定するmainまたはgpu-mapだけが所有できる。所有意図・leaseが古いウインドウからの操作を拒否する。
2. DOMの表示領域をResizeObserver・scroll・resize・focus・visibility・共有revision・表示条件と1秒の状態再確認により同期する。requestAnimationFrameで合流し、IPCは1件ずつ送り、待機中の変更は最新の位置と表示条件へまとめる。UIの条件／revisionトークンが変わった後の応答は件数表示へ採用せず、古い失敗で新しい表示を閉じない。
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

🟢 [Done] Rust所有ネイティブラベル、縦ズーム／hit-test、専用GPUパネルのフローティングを実装。2026-10-25節の検証境界を参照。

## 通常マップの表示条件との一致（2026-10-25）

🟢 [Done] `sync_gpu_panel`に必須のFilterを追加。検索語（UTF-16で最大512単位・NUL不可）、all／none／allow／block／ask、hideLoopback、live／demoだけを受け、未知フィールドや値を拒否する。検索窓も同じ上限でNULを除く。接続IDの一覧、署名、ルール、フレームや頂点をWebViewから送らない。Rustは現在のSessionから観測済み接続とルールを読み取る。FilterはRustのパネル描画キャッシュに保持する表示条件であり、保存文書・ルール・共有選択は変更しない。

- 検索は通常マップと同じアプリ名＋接続先＋実行パスへの小文字部分一致。OSリゾルバー結果・ポート・表示アイコンは検索へ混ぜない。独立フロー名は収集同意と検索の明示許可が両方ある場合だけ候補名を追加する。ソート順とアプリの展開状態はGPU概要の対象条件ではなく、既存の決定的な描画順序を維持する。
- 意図の選択は実際の適用／遮断と区別する。TCPだけ、source・正確な接続先／ポート・方向を照合し、実測ルールは検証済み署名主体とcanonical keyが必要。旧パスルールや他発行元は一致させない。具体的方向をbothより優先し、更新時刻、同率では先のルールを使う。これは通常マップのruleForと同じ表示上の優先順位。
- Mac内通信は127/8、::1、IPv4マップドIPv6を通常マップと同じ分類で隠す。角括弧・有効なzone・IPv4の先頭ゼロも照合する。LANをMac内として隠さず、localhostという名前をIPループバックと推測しない。
- revisionだけでなくFilterの一致もフレーム再利用の条件とする。条件が変われば同じrevisionでも再投影／描画する。条件外の選択IDをシーンへ強制挿入しない。0件も空のSurfaceをpresentして以前の線を消し、上限は条件一致した接続へ適用する。truncatedは全接続ではなく、条件一致件数が描画件数を超えた場合だけ表示する。
- 実測／デモ区分がRustの現在値と違えば拒否し、古い要求で別の区分を描かない。lease／sequenceで再オープン後や順序逆転を拒否する。UIは待機中の入力を最新値へ合流し、条件変更前の件数を新しい条件の件数として表示しない。すでにpresentされたフレームは次の要求が反映されるまで残り得る。入力とOS表示が完全に同時になることは保証しない。

🟢 [Done] `src/core/fixtures/gpu-filters.json`の14ケースをRustと通常マップの本物のruleFor／classifyAddressで検証。日本語／Unicode・パス検索、3種のループバック、LAN、具体的方向・更新時刻・同率優先、UDP、署名ルールと旧パスルール、条件の交差を含む。純粋テストに加え、実Metalの一時Tauriで空表示→全体へ復帰、同じrevisionでの条件キャッシュ失効、1件の意図・ルールなし・ループバック・上限の件数、source／sequence拒否を確認。

🟢 [Done] ブラウザーはIPCをモックし、3言語・1500／375px、フィルター切替、0件、遅延した成功／失敗の拒否、閉じる→再オープン、通常ブラウザーでの非対応表示を確認。ブラウザーの白い描画枠はMetalピクセルを証明しない。

⭕️ [Pending] 署名済み配布アプリでの実観測・実UI入力とMetalの通し確認、別OS／GPU、Retina／複数ディスプレイ・スリープ、長時間のフィルター連打とメモリ／電力。今回のコードは公証承認待ち0.1.1 ZIPへ含めていない。

## ラベル・camera・選択・浮動窓（2026-10-25）

🟢 [Done] 曲線はwgpu、アプリ名とIP:portは同じRust Controllerが所有するNSView内のネイティブNSTextFieldで描画。左右各最大32ラベル、重なり省略と全文tooltipを設ける。文字atlasはJavaScriptへ送らない。縦方向1～8倍zoom、上下pan、reset、線の32分割Bezierと同じ形状によるhit-testをRustで処理し、最前面の近い線を共有選択へ反映する。

🟢 [Done] 専用gpu-map Tauri窓へfloat／mainへdock。Rust Sessionに所有窓・filter・操作世代を保持し、cameraはControllerが窓切替をまたいで保持する。古いmainの同期・close、閉じたfloatの遅延応答で新しいSurfaceを作らない。描画データやフレームはIPCを通さず、命令・矩形・状態だけを渡す。

⭕️ [Pending] 署名済み配布版の通常WebView入力からの通し確認、Retina・複数画面・スリープ・長時間負荷。nativeラベルはwgpu文字レンダラーではない。詳細操作は通常Inspectorを利用する。

# macOS標準ファイアウォール用ヘルパー

2026-10-17。macOSの標準受信ファイアウォールと、FlowGuard Network Extensionによるアプリ別制御は独立している。

## 実装した経路

設定画面の「状態を確認」→確認チェック→ヘルパー登録→必要に応じ「承認画面を開く」→システム設定で管理者承認→状態を再確認→確認チェック→有効／無効→OSの管理者認証→変更後の再読み取り。

起動時の自動登録・変更・復元・再試行はしない。確認チェックは操作ごとに解除する。ヘルパー登録解除は標準ファイアウォールの設定を変更しない。無効化は受信保護を弱めるため、操作前の情報アイコンで説明する。認証情報・Authorization外部形式はUI／Rust／ディスク／ログに渡さない。

- Rust `firewall_control::Machine` が操作世代・busy・最後の読取結果を共有し、WebViewは確認チェックだけを保持する。同時操作とインストール中の操作を拒否し、操作中はアプリ更新インストールを拒否する。保存文書は変更しない。
- Rustは同梱 `Contents/MacOS/FlowGuardFirewallHelper` だけを起動する。引数は状態、登録、登録解除、承認設定画面、有効、無効と既知の期待状態に限定。認証待ちを含め最大120秒。異常終了・タイムアウトでは結果不明にし、再確認まで変更を認めない。
- CLIは自身の有効なTeam IDと、同じTeam IDの `com.flowguard.desktop` 親プロセスの署名を要求する。署名のない開発環境はServiceManagement／Authorization APIを呼ぶ前に拒否する。
- 登録はmacOS 13以降のSMAppService LaunchDaemon。固定ラベル／Machサービス名 `com.flowguard.firewall-helper`、BundleProgramは同梱バイナリ。起動はMachサービス要求時。root以外でdaemon処理は動かさない。
- XPCリスナーとクライアントは、Apple署名チェーン・同じTeam ID・ヘルパーの署名識別子を相互に要求する。最大4接続、接続寿命20秒、1接続1変更。root処理は全接続に対して直列化する。
- 変更のたびにCLIがAuthorization Servicesの `system.privilege.admin`（管理者実行権）を要求する（OSの認証キャッシュ／ポリシーにより認証UIが省略される場合がある）。daemonは外部形式の正確な長さと既存権限を検証し、権限の拡張や対話を行わない。CLIは完了・キャンセル時に権限を破棄する。
- daemonは待機後にも接続の有効性・15秒の単調時計期限・認証を確認する。状態読取後にも再確認し、切断・期限切れの待機要求を実行しない。すでに開始したOS処理の取り消しは保証しない。
- コマンドは固定の `/usr/libexec/ApplicationFirewall/socketfilterfw` の `--getglobalstate` と `--setglobalstate on|off` のみ。シェル、sudo、任意パス、アプリ別設定、任意文字列は受け取らない。環境を固定し、出力保持4096バイト、各実行5秒。期待値不一致・不明値では変更しない。
- 終了コードだけで変更成功とはしない。実行後に独立して再読取し、数値状態が目標と一致した場合だけ成功にする。未知の出力は不明値。接続喪失ではOS処理が完了した可能性があるため、成功／元に戻ったと断言しない。

## 配布

`python3 scripts/package-native.py` が未署名ステージングアプリへバイナリと `Contents/Library/LaunchDaemons/com.flowguard.firewall-helper.plist` を追加する。登録やOS設定変更はしない。

`scripts/release.py` はヘルパーをDeveloper ID・hardened runtime・明示的な `com.flowguard.firewall-helper` 識別子で署名してから外側のアプリを署名・検証する。observe-only配布には含めない。署名コードの追加と、署名済み配布物の実証は別。

## 検証範囲

🟢 [Done] Swiftコンパイル、固定コマンド、厳格な状態出力、認証／期待状態／切断を模擬した書込拒否と再読取。Rustの確認要求・共有ロック・更新インストール排他・世代検証・結果不明。3言語UI、ブラウザプレビューの操作無効化、1200／375pxでの表示。未署名ステージング生成。

⭕️ [Pending] このヘルパーを含むDeveloper ID署名・公証済み配布物、SMAppService登録・管理者承認・取り消し・再登録、実Authorization認証／キャンセル／認証待ちタイムアウト、異署名／異Team／直接CLIの実サービス拒否、実root XPCとsocketfilterfw変更・再読取、外部設定変更・OS処理中切断・OS再起動・アプリ移動／更新／削除後の挙動。開発検証では登録・管理者認証・標準ファイアウォール変更を実行していない。管理対象Macの認証ポリシーによる権限制限も実機確認対象。

## Appleの契約

- [SMAppService](https://developer.apple.com/documentation/servicemanagement/smappservice)
- [register(): LaunchDaemonは管理者承認後にbootstrap](https://developer.apple.com/documentation/servicemanagement/smappservice/register())
- [BundleProgramと同梱ヘルパーへの移行](https://developer.apple.com/documentation/servicemanagement/updating-helper-executables-from-earlier-versions-of-macos)

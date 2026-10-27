# 診断の根拠と影響

🟢 [Done] 既存のサービス確認結果を評価する際、各診断事項にRust側で `evidenceSource` と `impact` の固定コードを付ける。サービス応答、TLS検証、証明書メタデータ、HTTPヘッダー未収集を区別し、未確認のサービス・TLS、未取得の期限・ヘッダーを不明として表す。取得できた範囲だけを説明し、脆弱性や機器全体の安全性を推定しない。

詳細は取得元→観測根拠→考えられる影響をアイコン付きの3ノードで表示し、重要度と根拠の状態、元サービス操作・インターフェース・受信した観測時刻、推奨対応を併記する。狭幅は縦配置。新たな通信や設定変更は行わない。未対応コードや旧Snapshotでは取得元／影響を未確認として扱う。

診断CSV／HTMLの既存14列の末尾に `evidence_source` と `impact` の2列を追加。さらに診断比較機能で `baseline_report` を追加し、現在は17列。CSVは機械可読コードで、既存の共通CSVセル処理を使用し、引用符・改行と先頭空白を含む数式形式を保護する。HTMLも文字をエスケープし、追加列見出しは3言語。列数が変わるため固定列数を前提にする外部連携は更新が必要。

🟢 [Done] Rust 187試験（24 ignored）・UI 165試験、production build、cargo fmt --check、更新処理3件・リリースゲート3件。取得元／影響の状態ごとの区別とCSV出力・危険なセル・HTML文字エスケープを確認。Browser skill不在のため既存Playwright/Chromeを使用し、3言語 × 1500／768／375／320pxで選択したTLS／未収集HTTP・旧Snapshotの表示、外部文字列の非実行、横はみ出し・consoleを確認。QA /tmp/flowguard-diagnostic-detail-qa.cjs、画像 /tmp/flowguard-diagnostic-detail-{ja,en,zh-CN}-{1500,768,375,320}.png。

🟢 [Done] 別IDのdebug Tauri・実WebKit・日本語1440×960で、このMac自身のIPv4・51857番に一時HTTP応答を設け、TCP／HTTP確認から実診断を作成。HTTPヘッダー未実施の詳細で取得元→観測根拠→影響の3ノードと推奨対応を確認。実際に保存したCSV／HTMLは各2行・16列で内容が一致し、service_observation／service_scope_review、headers_not_collected／headers_unknownを確認。一時待受は停止。

⭕️ [Pending] 署名済み配布版、実機のTLS証明書・ネットワーク設定、実Tauriの他言語／狭幅。外部アドバイザリなどの情報源や実際の影響の確定は対象外。

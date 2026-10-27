# 診断の根拠と影響

🟢 [Done] 既存のサービス確認結果を評価する際、各診断事項にRust側で `evidenceSource` と `impact` の固定コードを付ける。サービス応答、TLS検証、証明書メタデータ、HTTPヘッダー未収集を区別し、未確認のサービス・TLS、未取得の期限・ヘッダーを不明として表す。取得できた範囲だけを説明し、脆弱性や機器全体の安全性を推定しない。

詳細は取得元→観測根拠→考えられる影響をアイコン付きの3ノードで表示し、重要度と根拠の状態、元サービス操作・インターフェース・受信した観測時刻、推奨対応を併記する。狭幅は縦配置。新たな通信や設定変更は行わない。未対応コードや旧Snapshotでは取得元／影響を未確認として扱う。

診断CSV／HTMLの既存14列の末尾に `evidence_source` と `impact` の2列を追加。さらに診断比較機能で `baseline_report` を追加し、現在は17列。CSVは機械可読コードで、既存の共通CSVセル処理を使用し、引用符・改行と先頭空白を含む数式形式を保護する。HTMLも文字をエスケープし、追加列見出しは3言語。列数が変わるため固定列数を前提にする外部連携は更新が必要。

🟢 [Done] Rust 187試験（24 ignored）・UI 165試験、production build、cargo fmt --check、更新処理3件・リリースゲート3件。取得元／影響の状態ごとの区別とCSV出力・危険なセル・HTML文字エスケープを確認。Browser skill不在のため既存Playwright/Chromeを使用し、3言語 × 1500／768／375／320pxで選択したTLS／未収集HTTP・旧Snapshotの表示、外部文字列の非実行、横はみ出し・consoleを確認。QA /tmp/flowguard-diagnostic-detail-qa.cjs、画像 /tmp/flowguard-diagnostic-detail-{ja,en,zh-CN}-{1500,768,375,320}.png。

🟢 [Done] 別IDのdebug Tauri・実WebKit・日本語1440×960で、このMac自身のIPv4・51857番に一時HTTP応答を設け、TCP／HTTP確認から実診断を作成。HTTPヘッダー未実施の詳細で取得元→観測根拠→影響の3ノードと推奨対応を確認。実際に保存したCSV／HTMLは各2行・16列で内容が一致し、service_observation／service_scope_review、headers_not_collected／headers_unknownを確認。一時待受は停止。

⭕️ [Pending] 署名済み配布版、実機のTLS証明書・ネットワーク設定、実Tauriの他言語／狭幅。外部アドバイザリなどの情報源や実際の影響の確定は対象外。

🟢 [Done] 完全で構文上有効な最終HTTP／HTTPS HEAD応答から、CSP・X-Content-Type-Options・Referrer-Policy・HSTSの非空値の有無を取得。値・Cookieは保存しない。診断の根拠ノードに各項目の検出／未検出を表示し、値の妥当性や設定全体の有効性は未確認と明記。暫定応答、不完全・不正な応答、未取得は未実施。HSTSはHTTPSで確認が必要。

🟢 [Done] Rust192件（24 ignored）・UI167件・production build。完全／不完全／不正／暫定応答と秘匿値を保持しないこと、診断とCSVへの反映を確認。Browser plugin不在のためPlaywright／Chromeと合成Snapshotで3言語 × 1500／768／375／320pxの詳細表示を確認。

🟢 [Done] 別IDのdebug Tauri・日本語1440×960で、このMac自身のHTTPサービスから診断。CSPとX-Content-Type-Optionsを検出、Referrer-PolicyとHSTSを未検出と表示し、取得元と有効性未確認の説明を確認。テスト用Cookieは画面に表示されなかった。

⭕️ [Pending] 実TauriでのHTTPS・実証明書・他言語／狭幅、署名済み配布版。ヘッダー値の正しさ、HTTPS誘導、他ページの確認は未実装。

🟢 [Done] HTTPの単一HEAD応答に対し、301／302／303／307／308のLocationからHTTPS・HTTP・相対参照・その他・判定不能を分類。応答コードと方式のみ共有し、生のURL・クエリは保存しない。複数Location、不正な参照、その他の3xxは判定不能。不完全応答は未実施。誘導先への接続やTLS検証は実行しない。診断に揃ったノードと限界を表示し、既存の比較・CSV／HTMLへ機械可読コードで反映。

🟢 [Done] Rust193件（24 ignored）・UI167件・production build。方式分類・不正な参照・複数Location・秘密情報を保持しないこと・未実施を検証。Browser plugin不在のためPlaywright／Chromeと合成Snapshotを使い3言語 × 1500／768／375／320pxでノードと判定不能・未実施を確認。

🟢 [Done] 別IDの実debug Tauri・日本語1440×960で、このMac自身のHTTP 301応答から「HTTP 301 → HTTPSへ誘導」、接続先・TLSの安全性未確認の説明を確認。テスト用URLとクエリは画面に表示されなかった。

⭕️ [Pending] 実Tauriでの他の誘導応答、他言語／狭幅のWebKit、署名済み配布版。実際の誘導先とHTTPSの接続・証明書・他ページの動作は未確認。

🟢 [Done] 完全なHTTP／HTTPS HEAD応答のX-Content-Type-Optionsについて、単一の大文字小文字を区別しない `nosniff`、ヘッダーなし、空／別の値、複数行またはカンマ区切りの判定不能を区別。未取得は未実施。生の値は保持せず、判定コードだけを共有状態・診断・CSVへ記録する。単一応答に限り、Content-Typeやブラウザ動作、他ページの保護を確認しない。[Fetch Standard](https://fetch.spec.whatwg.org/#x-content-type-options-header)を参照し、複数値は保守的に判定不能とする。

🟢 [Done] Rust195件（24 ignored）・UI167件・production build。値の分類・不完全応答・複数値・機密値非保持・差分と未取得を検証。Browser plugin不在のためPlaywright／Chromeの合成Snapshotで3言語 × 1500／768／375／320pxの診断詳細とサービス差分を確認。

⭕️ [Pending] 実TauriでのHTTP／HTTPS応答・CSV・複数ウインドウ・署名済み配布版。

# 診断履歴の比較

🟢 [Done] 比較元はセッション内の直近の一致する診断履歴。対象・送信元・インターフェース・ネットマスク・アドレス種別・LAN世代・ポート集合・確認方式・HTTPS名がすべて一致し、ポート集合と完了件数が整合する場合のみ選ぶ。ポート順は正規化し、重複や不足は比較しない。異なる条件の過去記録は比較元としない。入力値の文字列表記も一致条件に含め、同一IPの異表記では比較を見送る場合がある。

両方の根拠が観測済みの場合、根拠・重要度・取得元・影響を比較する。片方でも未確認／未実施の項目は比較不確定。前回にない観測済み項目は新規。比較元がない場合は比較なし。比較元のレポート番号は概要にアイコン付きで表示し、キーボードフォーカス／ツールチップで条件を確認できる。CSV／HTML末尾にもbaseline_report列を追加（計17列）。固定列数を前提とした外部連携の更新が必要。

🟢 [Done] Rust188件（24 ignored）・UI165件、production build、cargo fmt --check、リリースゲート3件。Rust試験で条件差・不完全な結果・未確認／未実施・観測済みの比較と出力見出しを検証。Browser skill不在のためPlaywright/Chrome、合成Snapshot／模擬IPCで3言語 × 1500／768／375／320pxの概要・比較元・既存フィルター・空状態・横はみ出し・consoleを検証。QA /tmp/flowguard-diagnostic-comparison-qa.cjs。

⭕️ [Pending] 実Tauriでの同条件再診断・条件変更・CSV／HTML保存、署名済み配布版・実ネットワーク変更。比較は取得済み情報だけであり、過去に観測されなかった項目や物理機器の同一性を保証しない。履歴はセッション内のみ。


🟢 [Done] 別IDのdebug Tauri・実WebKit・日本語で、このMacのIPv4単一ポートをHTTPで2回確認し、各結果から診断を作成。2回目は比較元 #1、サービス応答は「変化なし」、未実施HTTPヘッダーは「比較不確定」と表示された。一時HTTP待受は停止。

⭕️ [Pending] 続けて実ファイル出力を試みたが、Computer UseがnoWindowsAvailableを返し、アプリの再取得もtimeoutReachedとなった。比較元入りCSV／HTMLと条件変更時の実Tauri確認は未完了。アプリがクラッシュしたと断定できない。


🟢 [Done] 実Tauriの診断 #2（比較元 #1）をCSV／HTMLとして保存し、両方17列・2件・全セル一致、service=unchanged、http_headers=uncertain、baseline_report=1を確認。続けて同じ自己宛てHTTP待受へHTTPS方式で確認し、TLS接続失敗の診断 #3 は4件すべて比較なし・比較元なし。CSV／HTMLの全セル一致・baseline_report空欄を確認。一時待受は停止。

⭕️ [Pending] 署名済み配布版・実TLSサーバー・実ネットワーク切替・他言語／狭幅。

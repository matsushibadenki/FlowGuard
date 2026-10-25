# 「以降の計画」より前のLater完了作業

2026-10-25。ユーザーの指定は`ROADMAP.md`の「以降の計画」見出しより前にあるLaterのみ。以降のLAN拡張・SSH管理・AI・ルーター等は対象外。既存Pendingの実機確認は、実施できた根拠なしにDoneへ変更しない。

| ID | 対象 | 状態 |
| --- | --- | --- |
| L1 | Network Extensionのフロー名契約、複数名、検索 | 🟢 [Done] |
| L2 | UDPブリッジ・限定保護・管理一時許可 | 🟢 [Done] |
| L3 | 調査／サービス／診断の共通操作と関連移動 | 🟢 [Done] |
| L4 | 診断から既存のFirewall意図作成へ移動 | 🟢 [Done] |
| L5 | ドラッグによるドッキング／フローティング | 🟢 [Done] |
| L6 | GPUラベル・ズーム・接続選択・フローティング | 🟢 [Done] |
| L7 | 複数アーキテクチャ公開・永続更新再開・履歴／チャネル／鍵移行 | 🟢 [Done] |

順序は共通操作／関連移動→パネル→GPU→観測名→UDP→配布基盤。各項目で実装とテストを揃え、対象外の将来機能を実装済みと表示しない。外部サービスの公開・OS設定変更・実ネットワーク調査は自動実行しない。

## 実装と検証の対応

- L1: `flow_names.rs`／`FlowNames.swift`。既定オフ、明示15秒lease、フロー署名・全tuple・時刻の候補照合、複数名、検索の別許可。停止済みstoreは遅延要求を拒否。名前を信頼済みidentityにしない。
- L2: policy／ask／初期設定v5、runtime ABI5。TCP transportを明記し、旧文書だけは既知のTCP形式から読み込む。一般UDPルールは拒否。Appleリゾルバーの送信UDP DNSだけ初期フローで限定判定。独立一時許可は直近のProviderフローUUIDへ結び付け、署名・tuple・interface・network世代・単調期限を固定。5秒heartbeat、最長10分、個別／全解除、停止後の要求拒否、Provider再起動ごとの新store。完全一致するask待機フローの再開をテストし、別UUIDは拒否。すでに許可された通信の遡及遮断や、切断後の再接続許可は提供しない。
- L3/L4: `diagnostics.rs`と共通evidence sheet。既存の完了したサービス観測から限定診断、未実施の明記、セッション20履歴、差分・対応理由、エスケープ済み出力。古い探索世代／ポート操作からの移動を拒否。ルールは現在の署名済み接続が一意の場合だけ既存の確認・意図保存・別途適用画面へ移動する。
- L5: Inspectorの既存左／右／浮動配置に専用MIMEのdragを接続。キーボード・取消を用意。全パネル任意分割の新レイアウトエンジンではない。
- L6: 曲線・camera・hit-testはRust/wgpu、文字は同じRust Controllerが所有するネイティブラベル。縦ズーム／パン／共有選択・専用Tauri浮動窓とドック復帰。状態はRust Session、所有窓／lease／sequenceで古い窓からの復活を拒否。
- L7: ディスク200MB・排他・strong ETag限定resume、全候補の署名とMach-O CPU検証、履歴20件、stable/beta、同梱ルート鍵で認可された委任鍵。公開統合CLIはローカル生成が既定で、公開フラグは今回未実行。インストール要求と新版起動確認は別の履歴として保存。

## Pending

⭕️ [Pending] 署名済みProviderの実通信、フロー名取得条件・欠落、UDPの宛先変更・mDNSパケット、VPN／interface／スリープ、管理セッション継続と旧新版切替。peer根拠は利用者による元アプリでの確認であり、FlowGuardによるpeer認証ではない。ローカルIPのinterface対応を実パケットのegress証明と扱わない。

⭕️ [Pending] 実TauriのInspector跨ぎdrag、署名済み画面とGPU窓、Retina／複数ディスプレイ・長時間描画。隔離Metal試験とブラウザモックは実配布版の通し試験の代替ではない。

⭕️ [Pending] Intelビルド・両architectureの公証と公開、実HTTPS ETagのresume、本番鍵切替、beta公開、実アプリ更新と再起動。既存0.1.1 ZIPは更新せず、新版の署名／公証が必要。

## 最終結果

RustはGPUあり157件／なし153件、Swift43件、TypeScript120件、Python4件が成功。別途、ローカルHTTP resume1件と隔離Tauriの実Metal10項目が成功。Chromeで3言語×1500/375pxの診断／drag・名前／一時許可／更新・GPU操作を検証した。各23件のRust ignoredと実機保留はDoneに含めない。詳細は [VALIDATION.md](VALIDATION.md) の2026-10-25節。

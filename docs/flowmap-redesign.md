# 指示書: FlowGuard「通信マップ」画面のレイアウト・デザイン刷新

対象リポジトリ: `matsushibadenki/FlowGuard`(Tauri 2 + Rust + React 19 + TypeScript)
対象画面: `View = 'map'`(通信マップ)のみ
作業範囲: フロントエンド(`src/`)のみ。`src-tauri/`・`native/` は変更しない。

---

## 0. このタスクの目的

現状の通信マップは次の理由で「わかりにくい」。

1. 観測中の接続が172件あっても **6件しか描画しない**(`FlowMap.tsx` の `connections.slice(0,6)`)。残りは小さな「+166」の文字だけ。
2. 同じアプリが複数行に重複して並び、`127.0.0.1` が画面を占める。
3. 全ての線が中央の円に集まる同一の灰色破線で、**線が情報を持っていない**。
4. 最も重要な状態(「遮断は動作していない」)が画面下部の小さな文字にあり、緑色が「許可」「有効」「実際の通信」に使い回されて安心に見える。
5. 「ルールを保存」ボタンが、未適用でも遮断できたかのように読める。

これを解消する。ただし **FlowGuardの設計思想(観測・ルールの意図・OS適用を厳密に区別し、推測で補わない)は絶対に崩さない**。

---

## 1. 絶対に守る制約

### 1.1 意味論(最重要)

- 「観測」「ルールの意図(保存済み)」「OSへの適用(拡張機能の受領確認)」は別の状態として扱う。
- `applied` は「拡張機能がルール一式を受領した」ことであり、**個別通信の遮断の証明ではない**。この区別を文言から消さない(`docs/ARCHITECTURE.md` の "Observation is not enforcement" 参照)。
- 次の表現は、`extension.stage === 'applied'` を満たしていない状態では**画面のどこにも出さない**。
  「保護されています」「安全」「危険」「ブロック済み」「遮断中」「守られています」
- 次を**推測・追加表示しない**: ホスト名、国名、通信量、サービス名(ポート番号からの推定)、脅威判定、アプリの信頼度。
  追加してよいのは、IPアドレスの**範囲による分類**(§4.2)のみで、それも「範囲による分類」と明示する。
- 接続の方向は `Connection.direction`('unknown' を含む)をそのまま使う。推定で補わない。
- ルールの照合は既存の `ruleFor()`(`src/core/rules.ts`)を使い、**変更しない**。アプリ単位の包括ルールは存在しないので、アプリ行の「ルール状態」は個別接続の集計として表示し、「アプリ全体を遮断」のような表現にしない。

### 1.2 アーキテクチャ

- 状態の所有者はRust。`Snapshot` と `Command`(`src/core/types.ts`)を**変更しない**。新しいTauriコマンドを追加しない。
- 検索・絞り込み・並び順・展開状態は**プレゼンテーション状態**として `useState` で持つ。永続化しない(localStorageも使わない)。
- 選択の単一の真実は `snapshot.selectedId`(`dispatch({type:'select',id})`)。ローカルに別の「選択中接続」を持たない。
- 新規の外部依存を追加しない(仮想スクロールライブラリ、チャートライブラリ、アニメーションライブラリを含む)。アイコンは既存の `lucide-react` のみ。
- 他の画面(アプリ・ルール・履歴・設定・ネットワーク調査・サーバー確認・診断)の見た目と挙動を変えない。

### 1.3 品質・規約

- TypeScript strict と `noUnusedLocals`/`noUnusedParameters` を通すこと。
- 文言は `src/core/i18n.ts` の3言語(`en` / `ja` / `zh`)すべてに追加する。`ja`・`zh` は `Record<TextKey,string>` なので、キーの追加漏れはコンパイルエラーになる。
- 既存の `Intl`・`aria-*`・キーボードフォーカス・`prefers-reduced-motion` への配慮を維持する。
- **全てのコードファイルの先頭に、配置パスをコメントで書く**(TS/TSXは `// パス: src/...`、CSSは `/* パス: src/styles/map.css */`)。
- 既存ファイルを部分修正した場合は、最終報告で「ファイル名・変更箇所・変更内容」を明記する。
- コードを省略しない。新規ファイルが500行を超えそうなら、分割を提案して実行する。
- 非推奨のJavaScript API(`substr`、`escape`/`unescape`、`eval`、`document.write`、`navigator.platform`/`userAgent` など)を使わない。
- 既存コードは1行に詰めた書式が多い。**既存ファイルを丸ごと整形し直さない**。新規ファイルは読みやすい通常の書式で書く。

---

## 2. 現状の構造(把握しておくこと)

```text
App.tsx  (view==='map')
 ├─ <section className="overview surface">   ← 全Firewall画面で共通表示。今回マップのみ差し替える
 ├─ .connection-toolbar  (検索 input + ルール絞り込み select)   ← App のローカル state: search, filter
 └─ .map-workspace.panel-{left|right|floating}
     ├─ .map-main
     │   ├─ <FlowMap snapshot connections t select/>          ← 今回の主対象
     │   └─ <Connections ... 最近の通信(history先頭4件)/>
     └─ <Inspector .../>                                     ← ルール作成。軽微に変更(§7)
```

- `connections` は App で `search`/`filter` を適用済みの `Connection[]`。
- アプリ集約は `snapshot.applications`(`ApplicationGroup[]`、Rustが実行ファイルパス単位で投影)。`Applications.tsx` が `connectionIds` を `connections` と突き合わせる実装の先例になる。ブラウザデモ用の同等関数は `src/core/applications.ts` の `groupApplications`。
- 識別できないプロセスは `appKey` が空で、グループIDは `unknown:<pid>`。**これらを名前で統合しない**(別のまま残す)。
- ルールは `Rule`(appKey + destination + port + direction + action + source)。`ruleFor(rules, c, mode)` で個別接続に対する意図を得る。`ruleApplied(rule, extension)`(`src/core/enforcement.ts`)で受領確認済みかを得る。
- 色トークンは `tokens.css`。`--color-allow/-block/-ask`(各 `-soft` あり)は**ルールの意図専用**として扱う。

---

## 3. 新しい画面設計

### 3.1 レイアウト(デスクトップ幅 ≥ 1050px)

```text
┌ 状態バー ─────────────────────────────────────────────────────────────┐
│ 観測のみ — 遮断・確認は動作していません                    [実際の通信|デモ] │
│ ●─────○─────○─────○   172 接続 · 14 アプリ · ルール(意図) 0 件         │
│ 観測   ルール(意図) 拡張機能  適用確認                                   │
└───────────────────────────────────────────────────────────────────────┘
[🔍 アプリ名・接続先を検索]  [ルール: すべて▾]  並び順[接続数▾]  ☑ このMac内の通信を隠す

┌ アプリ (14) ──────────────┐ ┌ 選択中: Adobe Illustrator ───────────┐ ┌ 接続の詳細 ┐
│ ▣ Adobe Illustrator   12  │ │ インターネット (2)        [範囲による分類?] │ │ (既存      │
│   ▮▮▯▯▯ 意図あり 3/12     │ │  150.171.110.134 :443  TCP  ✓許可の意図 │ │  Inspector) │
│ ▣ Antigravity IDE…     8  │ │  17.253.x.x      :443  TCP  ルールなし  │ │            │
│   ルールなし              │ │ ローカルネットワーク (0)                 │ │            │
│ ▣ Safari               5  │ │ このMac内 (10)  ▸ 隠しています           │ │            │
│ …                         │ │                                       │ │            │
│ 凡例: ✓許可 ー遮断 ?確認   │ └───────────────────────────────────────┘ └────────────┘
└───────────────────────────┘
        最近の通信(既存の Connections をそのまま下に維持)
```

要点:

- **中央の円(FlowGuardハブ)と全接続の曲線を廃止**する。「アプリ → 接続先」の関係は、左のアプリ一覧と右の接続先ペインの**並置**で表す。
- 左ペイン = **アプリ単位に集約した一覧**(全アプリ。6件制限なし)。右ペイン = **選択中アプリの接続先**。
- 左右をつなぐ線は描かない(情報を持たないため)。選択中アプリ行と右ペインの見出しの内容を一致させ、選択行の強調(背景 + 左端の太線)で関係を示す。
- 右端に既存の Inspector を維持する。`panel-left` / `panel-right` / `floating` の既存の配置機構を壊さない。

### 3.2 レスポンシブ

- 既存のブレークポイント(1550 / 1250 / 1050 / 850 / 600)に合わせる。新しい任意のブレークポイントを増やさない。
- ≥1550px: Inspector 340px 固定。
- 851〜1549px: アプリペインとマップ接続先ペインは `minmax(240px,340px) minmax(0,1fr)`。
- ≤850px: 縦積み。アプリ一覧 → 接続先ペインの順。接続先ペインの見出しに選択中アプリ名を sticky 表示する。
- 検証幅は 320 / 375 / 414 / 768 / 1200 / 1550px(README の既存検証幅 + 大画面)。横スクロールがページ全体に出てはならない。

---

## 4. データ処理(純粋関数として実装し、vitest でテストする)

### 4.1 `src/core/protection.ts`(新規)

状態バーの内容を `Snapshot` から導出する純粋関数。

```ts
// パス: src/core/protection.ts
import type {Snapshot} from './types';
import type {TextKey} from './i18n';

export type ProtectionTone = 'neutral' | 'caution' | 'acknowledged';
export interface ProtectionState {
  headline: TextKey;      // 状態バーの見出し
  tone: ProtectionTone;
  reached: 0 | 1 | 2 | 3 | 4; // 到達済みステップ数(0=デモ/観測不可)
  steps: [boolean, boolean, boolean, boolean]; // 観測/ルール(意図)/拡張機能/適用確認
}
export function protectionState(snapshot: Snapshot): ProtectionState;
```

判定規則(上から順に最初に一致したもの):

| 条件 | headline | tone | steps |
|---|---|---|---|
| `mode==='demo'` | `protectionDemo` | neutral | すべて false |
| `firewall.error` が非 null | `protectionObserveFailed` | caution | すべて false |
| `extension.stage==='applied' && !extension.busy` | `protectionAcknowledged` | acknowledged | 4つとも true |
| stage が `stale` / `disconnected` / `failed` / `applying` / `initial_sync` / `disabling` / `removing` | `protectionUncertain` | caution | 観測=true、ルール=(liveルール数>0)、拡張機能=false、確認=false |
| それ以外 | `protectionObserveOnly` | neutral | 観測=true、ルール=(liveルール数>0)、拡張機能=(stage が `ready`)、確認=false |

- liveルール数は `snapshot.document.rules.filter(r=>r.source==='live').length`。
- `tone: 'acknowledged'` でも、画面に「安全」「ブロック済み」と書かない(§1.1)。見出しは `protectionAcknowledged` の文言のみ。

### 4.2 `src/core/destinations.ts`(新規)

IPアドレスの**範囲による分類**。ホスト名・国・組織は扱わない。

```ts
// パス: src/core/destinations.ts
export type DestinationClass = 'loopback' | 'local' | 'public' | 'other';
export function classifyAddress(address: string): DestinationClass;
```

規則:

- 前後の `[` `]` と、`%en0` のようなゾーン指定を取り除いてから判定する。
- IPv4: `127.0.0.0/8` → loopback。`10/8`・`172.16/12`・`192.168/16`・`169.254/16`・`100.64/10` → local。`0.0.0.0/8`・`224.0.0.0/4`・`240.0.0.0/4` → other。それ以外 → public。
- IPv6: `::1` → loopback。`fe80::/10`・`fc00::/7` → local。`::`・`ff00::/8` → other。`::ffff:a.b.c.d`(IPv4射影)は内側のIPv4で判定。`2000::/3` → public。その他 → other。
- パースできない文字列は `other`(例外を投げない)。
- 100.64/10 や local の扱いはあくまで「アドレス範囲による分類」。VPN(Tailscale等)経由の実経路は判別できないので、UIの説明文でそれを明示する(`classHelp`)。

テストに含める入力: `127.0.0.1`、`::1`、`[::1]`、`192.168.1.12`、`172.15.0.1`(public)、`172.16.0.1`(local)、`172.32.0.1`(public)、`100.64.0.1`、`169.254.1.1`、`fe80::1%en0`、`fd00::1`、`::ffff:10.0.0.1`、`2001:db8::1`、`150.171.110.134`、`224.0.0.251`、`not-an-ip`。

### 4.3 `src/core/mapModel.ts`(新規)

表示用モデルを組み立てる純粋関数。

```ts
// パス: src/core/mapModel.ts
import type {ApplicationGroup, Connection, ExtensionStatus, Rule} from './types';
import type {DestinationClass} from './destinations';

export type Intent = 'allow' | 'block' | 'ask' | 'none';
export interface DestinationRow {
  key: string;              // `${destination}|${port}|${protocol}|${direction}`
  destination: string;
  port: number;
  protocol: string;
  direction: string;
  klass: DestinationClass;
  connectionIds: string[];  // 同一キーに統合された接続(複数PID・複数ソケット)
  representativeId: string; // 選択に使う代表(最新observedAt)
  intent: Intent;
  acknowledged: boolean;    // ruleApplied() が true
}
export interface AppRow {
  id: string;               // ApplicationGroup.id
  app: string;
  appKey: string;
  processCount: number;
  connectionCount: number;
  byClass: Record<DestinationClass, number>;
  intentCounts: Record<Intent, number>; // 個別接続ごとの ruleFor 結果の集計
  attention: number;        // public かつ intent==='none' の接続数(並び替え用)
  destinations: DestinationRow[];
}
export type SortKey = 'connections' | 'name' | 'attention';
export function buildAppRows(args: {
  groups: ApplicationGroup[];
  connections: Connection[];   // App で search/filter 適用済み
  rules: Rule[];
  mode: 'live' | 'demo';
  extension: ExtensionStatus;
  hideLoopback: boolean;
  sort: SortKey;
  locale: string;
}): {rows: AppRow[]; hiddenLoopbackOnlyApps: number};
```

規則:

- `groups` の `connectionIds` を `connections` と突き合わせる(`Applications.tsx` と同じ。存在しないIDは捨てる)。接続が0件になったグループは除外する。
- **統合キー**は `destination|port|protocol|direction`。同じキーの接続は1行にまとめ、`connectionIds` に全IDを保持する。`representativeId` は `observedAt` が最大の接続。**ルールは appKey + destination + port + direction で決まるので、統合しても `ruleFor` の結果は同一**。
- `hideLoopback===true` のとき、loopback 行を除く。その結果、表示すべき行が0になるアプリは `rows` から外し、その数を `hiddenLoopbackOnlyApps` に入れる。
- `intentCounts` は統合前の**個別接続**を対象に `ruleFor(rules, c, mode)?.action ?? 'none'` で集計する(件数の意味を保つ)。
- 並び順: `connections`=接続数の降順(同数は `app.localeCompare(..., locale)`)、`name`=名前順、`attention`=`attention` 降順 → 接続数降順。
- 行内の `destinations` は、class(public → local → other → loopback の順)→ 接続数降順 → destination 昇順。
- テスト: 統合、IDの欠落、loopbackのみのアプリの除外、`appKey` が空のグループが名前で統合されないこと、`ruleFor` の方向規則(`both` フォールバックと `unknown` 方向)が保たれること。

---

## 5. コンポーネント設計

### 5.1 新規・変更ファイル一覧

| ファイル | 種別 | 役割 |
|---|---|---|
| `src/core/protection.ts` / `.test.ts` | 新規 | §4.1 |
| `src/core/destinations.ts` / `.test.ts` | 新規 | §4.2 |
| `src/core/mapModel.ts` / `.test.ts` | 新規 | §4.3 |
| `src/components/ProtectionStatus.tsx` | 新規 | 状態バー(§6.1) |
| `src/components/AppList.tsx` | 新規 | 左ペイン(§6.2) |
| `src/components/DestinationPane.tsx` | 新規 | 右ペイン(§6.3) |
| `src/components/IntentBar.tsx` | 新規 | 意図の積み上げバー(§6.2) |
| `src/components/FlowMap.tsx` | 全面書き換え | 上記を束ねる。**export名と props 型は維持**(§5.2) |
| `src/styles/map.css` | 新規 | マップ画面のCSS。`src/main.tsx` から import |
| `src/styles.css` | 一部削除 | 旧マップ専用ルールの削除のみ(§6.6) |
| `src/App.tsx` | 軽微 | 状態バーの差し替えのみ(§5.3) |
| `src/components/Inspector.tsx` | 軽微 | 保存ボタンの文言(§7) |
| `src/core/i18n.ts` | 追記 | §8 |
| `docs/ROADMAP.md` | 追記 | §10 |

### 5.2 `FlowMap`

```ts
export function FlowMap(props: {
  snapshot: Snapshot;
  connections: Connection[];
  t: T;
  select: (c: Connection) => void;
  clearFilters?: () => void; // 検索・絞り込みを解除する(App から渡す)
}): JSX.Element;
```

- ローカル state: `hideLoopback`(既定 `true`)、`sort`(既定 `'connections'`)、`expanded`(グループ別の展開状態)、`limits`(グループ別の「さらに表示」件数)。
- **フォーカス中のアプリ** = `snapshot.selectedId` を含む行。なければ先頭行(この場合 `dispatch` しない。表示だけ)。
- アプリ行クリック: そのアプリの**代表接続**を `select()` する。代表接続は「loopback以外のうち最新 `observedAt`、なければ最新」。これで Rust 側の `selectedId` が単一の真実のまま保たれ、フローティング詳細にも反映される。
- 接続先行クリック: その行の `representativeId` の接続を `select()`。
- 選択中の接続が、現在のフィルターで見えない行にあっても、Inspector の表示は変えない(既存どおり `snapshot.selectedId` から)。

### 5.3 `App.tsx` の変更(最小限)

- `{!tools&&<section className="overview surface">…}` を、`p.view==='map'` のときは `<ProtectionStatus snapshot={snapshot} t={t} native={native} onError={onError}/>` に置き換える。**他のview(apps/rules/history/settings)では従来の overview をそのまま表示**する。
- ProtectionStatus は、従来の overview が持っていた機能を**すべて引き継ぐ**: 実際の通信/デモの切り替え(`dispatch({type:'setMode',...})`、`!native` のとき live ボタンは無効)、接続数、保存ルール数。
- `<FlowMap ... clearFilters={()=>{setSearch('');setFilter('all');}}/>` を渡す。
- 検索 input と絞り込み select は既存の `.connection-toolbar` を使い続ける(App のまま)。並び順・「このMac内の通信を隠す」は FlowMap 内のツールバー行に置く。

---

## 6. 視覚デザイン仕様

### 6.1 状態バー(`ProtectionStatus`)— この画面で唯一の「記憶に残る要素」

- 左: 見出し(16px / 太字 / `--color-ink`)。見出しの文言は `protectionState().headline`。補助行に 12px で「保存したルールは、拡張機能が有効で適用が確認されるまでOSには適用されません」(`stepHelp`)。ただし tone が `acknowledged` のときは、見出しの下に「個別の通信の遮断を保証するものではありません」を常に出す(`protectionAcknowledged` に含める)。
- 中: **4段階トラック**(観測 / ルール(意図) / 拡張機能 / 適用確認)。到達済みは `--color-accent` の塗りつぶし円、未到達は `--color-rule-strong` の輪郭円、間は1pxの線。ラベルは各円の下(12px)。トラックは情報を持つ連続した手順なので、番号付けは不要(円+ラベルのみ)。`aria-label` に「4段階中N段階まで到達」を入れ、各段階に `title` で意味を説明する。
- 右: 実際の通信/デモの切り替え(既存の `.source-switch` のスタイルを再利用)と、数値3つ(接続 / アプリ / ルール(意図))。数値は `font-variant-numeric: tabular-nums`。
- **色の規則**: neutral = `--color-ink` + 中立の円。caution = `--color-ask`(橙)の左端4pxバー + 警告アイコン(`Info`/`AlertTriangle` ではなく既存の `Info` を使う)。acknowledged = `--color-accent`(青緑)。**`--color-allow`(緑)を状態バーで使わない**。
- 状態バーは常に画面最上部、sticky にしない。高さは最大 96px(1行見出し + トラック)。

### 6.2 アプリ一覧(`AppList` / `IntentBar`)

- 行は**カードにしない**。1つの面(`.surface`)の中に、1pxの区切り線(`--color-rule`)で行を並べる。
- 1行 = 高さ56px: `AppIcon` / アプリ名(`font-weight:600`、中央省略 `text-overflow` ではなく **先頭と末尾を残す**省略。CSSだけで難しい場合は末尾省略 + 全名を `title` に入れる) / 右端に接続数(`tabular-nums`)。
- 2行目(12px、`--color-muted`): `IntentBar` + 「意図あり 3/12」。意図が1つもないときはバーを出さず「ルールなし」(`noRule`)のみ。
- `IntentBar`: 高さ6px、幅 72px、角丸 3px。`intentCounts` の割合で allow / block / ask / none を積み上げる(`--color-allow` / `--color-block` / `--color-ask` / `--color-rule-strong`)。`role="img"` と `aria-label`(各件数を文字で)を付ける。**色だけに頼らず**、行の下に文字で件数を出しているので情報は重複する。
- 選択行: 背景 `--color-accent-soft`、左端3pxに `--color-accent`。`aria-current="true"`。
- 行は `<button>` ではなく `<ul>`/`<li>` 内の `<button>`。ホバーは背景の微変化のみ(`--dur-micro`)。
- 一覧の上に見出し「アプリ (N)」。一覧の下(スクロール領域の外)に、`hiddenLoopbackOnlyApps>0` のとき「このMac内だけで通信している N 個のアプリを隠しています [表示する]」(`hiddenApps`)。
- 凡例(許可の意図 / 遮断の意図 / 確認の意図)は、**アイコン + 文字**で、右ペイン見出しの下に1行で置く。
- 一覧の最大高さは `calc(100vh - 360px)` 程度で内部スクロール。ページ全体は縦に伸ばしすぎない。

### 6.3 接続先ペイン(`DestinationPane`)

- 見出し: 選択中アプリの `AppIcon` + 名前(18px)+ `appKey`(12px、`--font-code`、省略+`title`)+ 「N 接続 · M プロセス」。
- 本文は class ごとのグループ(`インターネット` → `ローカルネットワーク` → `その他` → `このMac内`)。各グループ見出しは「名前 (件数)」+ 折りたたみボタン(`aria-expanded`)。右端の `?` ボタンで `classHelp` を表示する(`details` 要素でよい)。
- 展開の既定: インターネット・ローカルネットワーク・その他は展開、このMac内は折りたたみ。
- 接続先行(高さ44px): 左にルールの意図を示す**縦の細いバー**(4px、意図の色)、`destination:port`(`--font-code`、`tabular-nums`)、`protocol`(12px)、方向(既存の `DirectionIcon` + 文言。`unknown` は「方向未判定」)、右端に意図チップ。
- **意図チップ**: アイコン(既存の `ActionIcon`)+ 文言(「許可の意図」「遮断の意図」「確認の意図」)。`acknowledged` なら末尾に小さく「適用確認済み」(既存キー `applied`)と `Check` を出す。`none` はチップを出さず、「ルールなし」を `--color-muted` の文字で出す。
- 統合行(`connectionIds.length>1`)は、行の右に「×N」(`mergedConnections`)。
- 選択行: 背景 `--color-accent-soft`、`aria-pressed="true"`。`representativeId` が `selectedId` か、`connectionIds` に `selectedId` を含む行。
- 1グループあたり最初の **50行**だけ描画し、「さらに N 件を表示」(`showMore`)で50件ずつ追加。仮想スクロールは使わない。
- 空の状態: アプリ未選択 `pickApp`。フィルターで0件のとき `noMatches` + `clearFilters` ボタン(`clearFilters` が渡されているときのみ)。接続が本当に0件のときは既存の `<Empty/>`。

### 6.4 タイポグラフィ・余白・形(`tokens.css` の既存トークンのみ使用)

- 書体: 既存(`--font-body` / `--font-code`)。**新しい書体を追加しない**。IPアドレス・ポート・パスは `--font-code`。
- 文字サイズ: `--text-xs`(11)/`--text-sm`(12)/`--text-body`(14)/`--text-lg`(16)/`--text-xl`(18)。見出しは `--text-lg`〜`--text-xl` まで。`--text-display` は使わない。
- 余白は `--space-*` のみ。ペイン間は `--space-md`。行の内側は左右 `--space-md`。
- 角丸: ペイン外枠 `--radius-lg`、行・ボタン `--radius-sm`。**全要素を同じ角丸のカードにしない**。
- 影: 既存の `--color-shadow` を外枠にのみ使う。行には影をつけない。
- 禁止(テンプレート感が出るため): 見出しの上の全大文字ラベル、文字列末尾の `→`、`A · B · C` のような中黒の羅列(既存の `観測済み · TCP` を新UIに持ち込まない)、グラデーションの装飾、行ごとのホバー拡大アニメーション。
- 動きは「ユーザー操作への応答」だけ(展開/折りたたみ、選択の強調)。`--dur-micro`〜`--dur-short` と既存の `--ease-out`。ページ読み込み時の演出は付けない。`prefers-reduced-motion` で無効化(既存のグローバル指定が効くこと)。

### 6.5 アクセシビリティ(必須)

- 色だけで状態を伝えない(全ての意図表示にアイコンと文字)。
- 文字コントラスト 4.5:1 以上(`--color-muted` を `--color-paper` 上で使う場合に特に確認)。
- クリック対象の高さは 32px 以上。
- フォーカスリングは `--color-focus` で常に見える。
- アプリ一覧は ↑/↓ で行間を移動できること(roving tabindex)。Enter/Space で選択。
- 状態バーの見出しは `role="status"`、`aria-live="polite"`(状態が変わったとき読み上げる)。

### 6.6 CSS の扱い

- 新規CSSは `src/styles/map.css` に書き、`src/main.tsx` で `import './styles/map.css'` する(`styles.css` の後)。
- 旧マップ専用のセレクタ(`.map-head` `.map-canvas` `.routes` `.route*` `.map-nodes` `.map-node` `.map-center` `.center-orbit` `.map-destinations` `.destination-node` `.destination-icon` `.map-footer`(使わなくなる場合)`.map-overflow` など)は、**参照が0件であることを `grep` で確認してから** `styles.css` から削除する。他のviewが使っているセレクタ(`.legend`、`.legend-dot`、`.notice`、`.surface`、`.source-switch` など)は残す。
- `styles.css` は1行が非常に長い。**削除は該当ルールだけに限定**し、他のルールの再整形をしない。差分が読めなくなる変更は禁止。

---

## 7. Inspector の軽微な変更

- 保存ボタンの文言: `snapshot.document.preferences.mode==='live' && snapshot.extension.stage==='applied'` のときだけ従来の `saveRule`。**それ以外は `saveIntent`(「意図を保存(未適用)」)**。保存中は従来どおり `saving`。
- ボタン直下の状態表示(`save-status`)は、文字サイズを `--text-sm` に上げ、`未適用` のとき `--color-muted` ではなく `--color-ink` にして見落としにくくする(`aria-live="polite"` は維持)。
- それ以外のInspectorの構造・ロジックは**変更しない**。方向の選択肢と `saveUnknown` の説明も維持する。

---

## 8. 追加する文言(`src/core/i18n.ts`)

ja が正。自然な言い回しへの調整は可だが、**意味(特に「遮断していない」「意図」「範囲による分類」)を変えない**こと。`{n}` `{m}` は `.replace()` で置換する(複数形処理は不要)。

| キー | ja | en | zh-CN |
|---|---|---|---|
| `protectionObserveOnly` | 観測のみ — 遮断・確認は動作していません | Observation only — blocking and prompts are not active | 仅观测 — 未启用阻止与确认 |
| `protectionDemo` | デモ表示中 — 実際の通信ではありません | Demo data — not your real traffic | 演示数据 — 并非实际通信 |
| `protectionObserveFailed` | 観測できていません | Observation is not working | 无法观测 |
| `protectionUncertain` | 適用状態を確認できません | The applied state cannot be confirmed | 无法确认应用状态 |
| `protectionAcknowledged` | 拡張機能がルールの受領を確認しました。個別の通信の遮断を保証するものではありません | The extension acknowledged the rules. This does not guarantee that any single connection was blocked | 扩展已确认收到规则。这不保证某个连接已被阻止 |
| `stepObserve` | 観測 | Observe | 观测 |
| `stepIntent` | ルール(意図) | Rules (intent) | 规则（意图） |
| `stepExtension` | 拡張機能 | Extension | 扩展 |
| `stepAcknowledged` | 適用確認 | Acknowledged | 应用确认 |
| `stepHelp` | 保存したルールは、拡張機能が有効で適用が確認されるまでOSには適用されません。 | Saved rules are not applied by the OS until the extension is active and acknowledges them. | 在扩展启用并确认之前，已保存的规则不会由系统应用。 |
| `appsShort` | アプリ | Apps | 应用 |
| `sortBy` | 並び順 | Sort | 排序 |
| `sortConnections` | 接続数の多い順 | Most connections | 连接数最多 |
| `sortName` | 名前順 | Name | 名称 |
| `sortAttention` | ルール未設定のインターネット接続が多い順 | Most unruled internet connections | 未设规则的互联网连接最多 |
| `hideLoopback` | このMac内の通信を隠す | Hide traffic within this Mac | 隐藏本机内部通信 |
| `hiddenApps` | このMac内だけで通信している{n}個のアプリを隠しています | Hiding {n} apps that only talk within this Mac | 已隐藏 {n} 个仅在本机内通信的应用 |
| `showHidden` | 表示する | Show | 显示 |
| `classLoopback` | このMac内 | Within this Mac | 本机内部 |
| `classLocal` | ローカルネットワーク | Local network | 本地网络 |
| `classPublic` | インターネット | Internet | 互联网 |
| `classOther` | その他 | Other | 其他 |
| `classHelp` | アドレスの範囲による分類です。VPNやプロキシ経由の実際の経路、接続先の組織や国は判別していません。 | Grouped by address range only. Actual routes through a VPN or proxy, and the destination's owner or country, are not determined. | 仅按地址范围分类。不判断经由VPN或代理的实际路径，也不判断目标所属组织或国家。 |
| `pickApp` | アプリを選ぶと、接続先を表示します。 | Choose an app to see its destinations. | 选择应用以查看其连接目标。 |
| `showMore` | さらに{n}件を表示 | Show {n} more | 再显示 {n} 项 |
| `mergedConnections` | ×{n} | ×{n} | ×{n} |
| `intentCount` | 意図あり {n}/{m} | Intent set {n}/{m} | 已设意图 {n}/{m} |
| `noMatches` | 条件に一致するアプリがありません | No apps match. | 没有匹配的应用 |
| `clearFilters` | 検索と絞り込みを解除 | Clear search and filters | 清除搜索和筛选 |
| `saveIntent` | 意図を保存(未適用) | Save intent (not applied) | 保存意图（未应用） |

既存キー(`allowIntent` / `blockIntent` / `askIntent` / `noRule` / `applied` / `unknownDirection` / `connections` / `savedRules` / `live` / `demo` / `noIdentity` など)は再利用する。重複キーを作らない。

---

## 9. 検証(作業完了の条件)

必ず次を実行し、結果を最終報告に貼ること。

```sh
npm ci
npm run build   # tsc -b && vite build がエラー0
npm test        # 既存テスト + 新規テストがすべて成功
git diff --stat # src-tauri/ と native/ に差分がないことを確認
```

### 9.1 受け入れ基準(すべて満たすこと)

1. 172件の接続を想定しても、全アプリが一覧に出る(6件制限がない)。
2. 同じ実行ファイルのアプリが重複行にならない。`appKey` が空のプロセスは名前で統合されない。
3. `127.0.0.1` / `::1` は既定で隠れ、「隠しています」の件数と解除操作が表示される。
4. 状態バーに、`stage` が `applied` 以外のとき「遮断・確認は動作していません」または「確認できません」が必ず出る。緑色は状態バーに使われていない。
5. 「安全」「保護されています」「ブロック済み」「遮断中」等の語が、`applied` 以外の状態で画面のどこにも現れない(`grep` で確認)。
6. 意図チップ・凡例は、すべてアイコンと文字を併用している。
7. 保存ボタンは、live かつ `applied` 以外では「意図を保存(未適用)」になる。
8. 検索・ルール絞り込みが従来どおり動作する(絞り込み後も統合・並び順が正しい)。
9. アプリ行/接続先行の選択で `snapshot.selectedId` が更新され、Inspector とフローティング詳細(`?panel=inspector`)が追従する。
10. 日本語・英語・简体中文で、文言が欠けず、レイアウトが崩れない。
11. 320 / 375 / 414 / 768 / 1200 / 1550px で、ページ全体に横スクロールが出ない。
12. キーボードのみで、アプリ選択 → 接続先選択 → ルール作成まで操作できる。

### 9.2 ビジュアル確認

- `npm run dev`(http://127.0.0.1:1420/)のブラウザデモで確認する。デモは4件しか接続がないので、**確認用に一時的な多件数データ**(60件以上、loopback・IPv6・長いアプリ名・同一アプリの複数PID・同一接続先の重複を含む)を `sampleConnections()` の呼び出し側で差し込んで確認する。**この一時データはコミットしない**(`docs/ARCHITECTURE.md` の "visual QA fixtures are temporary and not shipped" に従う)。
- 上記の幅ごとのスクリーンショットを撮り、最終報告に添える。
- ネイティブのTauriアプリ(macOS)での確認ができない環境では、その旨を明記し、ROADMAP には `⭕️ [Pending]` として記録する。**確認していないことを Done にしない**。

---

## 10. ドキュメント更新

`docs/ROADMAP.md` の「1. Firewall・共通基盤」に、次の形式で追記する(日付・絵文字・状態表記は既存に合わせる)。

- 🟢 [Done] 通信マップのレイアウト刷新(アプリ単位の集約一覧 + 選択アプリの接続先ペイン、状態バー、ループバックの既定非表示、範囲による接続先分類)。ブラウザデモと自動テストで確認済みの範囲のみ記載。
- ⭕️ [Pending] ネイティブアプリ(Apple Silicon)での実表示、および172件規模の実測データでの確認。
- 🔴 [Later] 接続先のホスト名表示(オプトイン方式の検討)、ドラッグによる自由ドッキング、メニューバー常駐と通知。

---

## 11. 今回やらないこと(スコープ外)

- Rust・Network Extension・IPC・`Snapshot`/`Command` の変更
- ホスト名解決、国名・組織名・通信量の表示、脅威判定
- アプリ単位の包括ルール(新しいルールモデル)
- 通信の流れを動かすアニメーション、GPU描画(`wgpu`)との接続
- 他のview(アプリ・ルール・履歴・設定・ネットワーク調査・サーバー確認・診断)の見た目の変更
- メニューバー常駐、通知、オンボーディング

---

## 12. 作業手順(この順で進めること)

1. `protection.ts` / `destinations.ts` / `mapModel.ts` と各テストを書き、`npm test` を通す。
2. `i18n.ts` に文言を追加(`npm run build` で3言語の欠落がないことを確認)。
3. `IntentBar` → `AppList` → `DestinationPane` → `ProtectionStatus` → `FlowMap` を実装。
4. `src/styles/map.css` を作成し、`main.tsx` で import。旧CSSは参照0件を確認してから削除。
5. `App.tsx` に最小限の配線を入れる。`Inspector.tsx` の保存ボタンを変更。
6. §9 の検証を実施し、必要なら修正。
7. `docs/ROADMAP.md` を更新。
8. 最終報告: 変更ファイル一覧(新規/修正/削除)、各修正ファイルの「場所・内容」、検証結果、スクリーンショット、未確認事項(Pending)を明記する。

途中で仕様に矛盾や不明点を見つけたら、**推測で実装せず**、その箇所と選択肢を最終報告に書くこと。

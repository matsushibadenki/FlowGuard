// パス: src/core/traffic-graph.ts
import type {DestinationRow} from './mapModel';
import type {Snapshot} from './types';
import {ruleFor} from './rules';
import {systemProtected} from './systemProtection';
export type ReceiptState = 'demo' | 'none' | 'confirmed' | 'unconfirmed' | 'protected';
export function destinationReceipt(row: DestinationRow, snapshot: Snapshot): ReceiptState {
  if (snapshot.document.preferences.mode === 'demo') return 'demo';
  if (row.intent === 'none') return 'none';
  const connection = snapshot.connections.find(c => c.id === row.representativeId);
  const rule = connection && ruleFor(snapshot.document.rules, connection, 'live');
  if (rule && rule.action !== 'allow' && systemProtected(rule)) return 'protected';
  return row.acknowledged && snapshot.firewall.error === null ? 'confirmed' : 'unconfirmed';
}
export function pageWindow<T>(rows: T[], page: number, size: number) {
  const pages = Math.max(1, Math.ceil(rows.length / size));
  const current = Math.max(0, Math.min(page, pages - 1));
  return {rows: rows.slice(current * size, (current + 1) * size), current, pages, start: current * size};
}
export const trafficGraphWords = {
  ja: {graph:'ノードグラフ',list:'一覧',apps:'アプリ',destinations:'接続先',connections:'接続記録',intent:'保存した意図',receipt:'適用状態',confirmed:'ルール受領確認済み',unconfirmed:'適用未確認',demo:'デモ・未適用',none:'対象ルールなし',protected:'保護プリセット対象',previous:'前へ',next:'次へ',page:'ページ',follow:'選択中へ',more:'接続先のページ',summary:'表示条件内の集計',noRule:'ルールなし',help:'線の色は保存したルールの意図です。実線は該当ルールの受領確認、破線は未確認を示します。矢印は観測された接続方向で、方向不明には矢印を付けません。線の太さは通信量を表しません。受領確認は個別通信の許可・遮断の証明ではなく、「確認」は確認待ちの実績ではありません。',lineConfirmed:'実線：ルール受領確認',linePending:'破線：受領未確認',select:'詳細を開く',sockets:'接続',processes:'プロセス',filtered:'表示中',scope:'範囲による分類',preview:'デモの観測データ'},
  en: {graph:'Node graph',list:'List',apps:'Apps',destinations:'Destinations',connections:'Connection records',intent:'Saved intent',receipt:'Application state',confirmed:'Rule acknowledged',unconfirmed:'Application unconfirmed',demo:'Demo · not applied',none:'No matching rule',protected:'System protection preset',previous:'Previous',next:'Next',page:'Page',follow:'Show selected',more:'Destination pages',summary:'Within current filters',noRule:'No rule',help:'Line color shows saved rule intent. Solid lines indicate acknowledgement of that rule; dashed lines indicate no confirmation. Arrows follow observed connection direction; unknown directions have no arrow. Width does not represent traffic volume. Acknowledgement does not prove an individual connection was allowed or blocked. Ask is an intent, not evidence of a pending prompt.',lineConfirmed:'Solid: rule acknowledged',linePending:'Dashed: unconfirmed',select:'Open details',sockets:'connections',processes:'processes',filtered:'Showing',scope:'Address-range category',preview:'Demo observations'},
  'zh-CN': {graph:'节点图',list:'列表',apps:'应用',destinations:'连接目标',connections:'连接记录',intent:'已保存意图',receipt:'应用状态',confirmed:'规则接收已确认',unconfirmed:'应用未确认',demo:'演示・未应用',none:'无匹配规则',protected:'系统保护预设对象',previous:'上一页',next:'下一页',page:'页',follow:'查看所选',more:'目标分页',summary:'当前筛选内的统计',noRule:'无规则',help:'线条颜色表示已保存的规则意图。实线表示该规则已确认接收，虚线表示未确认。箭头依据观测到的连接方向，方向未知时不显示箭头。线宽不表示流量。接收确认不证明某条通信已允许或阻止；询问是规则意图，不表示当前正在等待确认。',lineConfirmed:'实线：规则接收确认',linePending:'虚线：未确认接收',select:'打开详情',sockets:'连接',processes:'进程',filtered:'显示',scope:'地址范围分类',preview:'演示观测数据'}
};

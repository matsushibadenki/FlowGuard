import type {Connection} from './types';
export interface HistoryBucket {start:number;end:number;count:number}
/** Counts retained first observations, never attempts, bytes, or activity per second. */
export function historyTimeline(rows:Connection[]):HistoryBucket[]{
 const times=rows.map(row=>row.observedAt).filter(time=>Number.isFinite(time)&&time>=0);
 if(!times.length)return [];
 const first=Math.min(...times),last=Math.max(...times);
 const width=[60,300,900,3600,21600,86400].find(step=>Math.floor(last/step)-Math.floor(first/step)<12)??Math.ceil((last-first+1)/11);
 const start=Math.floor(first/width)*width;
 const buckets=Array.from({length:Math.floor((last-start)/width)+1},(_,i)=>({start:start+i*width,end:start+(i+1)*width,count:0}));
 times.forEach(time=>buckets[Math.floor((time-start)/width)].count++);
 return buckets;
}
export function inHistoryRange(row:Connection,range:{start:number;end:number}|null){return !range||(row.observedAt>=range.start&&row.observedAt<range.end);}
export const timelineWords={
 ja:{title:'初回観測の分布',records:'記録',apps:'アプリ',destinations:'接続先IP',all:'すべての時間',empty:'保持中の履歴はありません',help:'検索に一致する保持中の行を、初回観測時刻で集計します。棒を選ぶと一覧を絞り込みます。通信量・接続試行数・継続時間ではありません。短命接続や保持期限外の行は含まず、空の時間帯も通信なしを意味しません。',demo:'デモ',range:'選択時間帯'},
 en:{title:'First observations',records:'Records',apps:'Apps',destinations:'Destination IPs',all:'All times',empty:'No retained history',help:'Counts retained rows matching search by first observation time. Select a bar to filter the list. These are not bytes, connection attempts, or durations. Short-lived connections and expired rows are excluded; empty intervals do not mean no traffic.',demo:'Demo',range:'Selected interval'},
 'zh-CN':{title:'首次观测分布',records:'记录',apps:'应用',destinations:'目标IP',all:'全部时间',empty:'暂无保留的历史',help:'按首次观测时间统计符合搜索条件的保留记录。选择柱形可筛选列表。这不是流量、连接尝试次数或持续时间，不包含短暂连接及已过期记录。空白时段不代表没有通信。',demo:'演示',range:'所选时段'}
};

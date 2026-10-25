import type {Connection} from './types';
export const historyRetentions=['session','off','15m','1h','24h'] as const;
export type HistoryRetention=typeof historyRetentions[number];
export function validHistoryRetention(value:unknown):value is HistoryRetention{return historyRetentions.includes(value as HistoryRetention);}
/** Browser preview only; the native source of truth and expiry stay in Rust. */
export function retainedHistory(rows:Connection[],policy:HistoryRetention,now:number):Connection[]{
 if(policy==='off')return [];
 const seconds=policy==='15m'?900:policy==='1h'?3600:policy==='24h'?86400:null;
 return rows.filter(c=>seconds===null||(c.observedAt<=now&&now-c.observedAt<seconds)).slice(0,500);
}
export const historyWords={
 ja:{title:'通信履歴',retention:'保持期間',session:'セッション中（既定）',off:'残さない',minutes15:'15分',hour1:'1時間',hours24:'24時間',scope:'保持期間について',help:'初回観測時刻からの期間内、最大500件をメモリに保持します。アプリ終了時に消去し、履歴をディスクに保存しません。ウインドウを閉じるだけではアプリは終了しません。短縮や「残さない」で削除した行は戻せません。現在の通信・ルール・保護状態、他の調査結果は変更しません。「残さない」でも通信の観測は続きます。再度有効にした後は、新しい接続から記録します。',preview:'デモ履歴',count:'{n}件・最大500件',sessionOnly:'終了時に消去',error:'保持期間を保存できませんでした。'},
 en:{title:'Connection history',retention:'Retention',session:'This session (default)',off:'Do not retain',minutes15:'15 minutes',hour1:'1 hour',hours24:'24 hours',scope:'About retention',help:'Keep up to 500 rows in memory, measured from first observation. History is cleared when the app exits and is never saved to disk. Closing a window does not quit the app. Shortening retention or disabling it permanently removes affected rows. Current connections, rules, protection and other investigation results are unchanged. Observation continues when retention is off. After re-enabling, new connections are recorded.',preview:'Demo history',count:'{n} rows · maximum 500',sessionOnly:'Cleared on exit',error:'Could not save retention.'},
 'zh-CN':{title:'连接历史',retention:'保留时间',session:'本次会话（默认）',off:'不保留',minutes15:'15分钟',hour1:'1小时',hours24:'24小时',scope:'关于保留时间',help:'自首次观测起，在内存中保留指定时间内的最多500条记录。退出应用时清除，历史不会保存到磁盘。仅关闭窗口不会退出应用。缩短时间或停用后删除的记录无法恢复。当前连接、规则、防护状态和其他调查结果不受影响。不保留时仍继续观测；重新启用后从新连接开始记录。',preview:'演示历史',count:'{n}条・最多500条',sessionOnly:'退出时清除',error:'无法保存保留时间。'}
};

import {useState} from 'react';
import {Clock,Info} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {dispatch} from '../core/store';
import {historyWords,type HistoryRetention as Policy} from '../core/history';
export function HistoryRetention({snapshot,compact=false}:{snapshot:Snapshot;compact?:boolean}){
 const p=snapshot.document.preferences,w=historyWords[p.locale];
 const [busy,setBusy]=useState(false),[error,setError]=useState(false);
 async function change(retention:Policy){setBusy(true);setError(false);try{await dispatch({type:'setHistoryRetention',retention});}catch{setError(true);}finally{setBusy(false);}}
 return <div className={`history-retention ${compact?'compact':''}`}>
 {!compact&&<div className="settings-heading"><Clock size={22}/><h2>{w.title}</h2></div>}
 <div className="history-retention-row"><label htmlFor={compact?'history-retention':'settings-history-retention'}>{w.retention}</label><select id={compact?'history-retention':'settings-history-retention'} value={p.historyRetention??'session'} disabled={busy||snapshot.updates.stage==='installing'} onChange={e=>void change(e.target.value as Policy)}><option value="session">{w.session}</option><option value="off">{w.off}</option><option value="15m">{w.minutes15}</option><option value="1h">{w.hour1}</option><option value="24h">{w.hours24}</option></select><span className="protection-info" tabIndex={0} title={w.help} aria-label={`${w.scope}: ${w.help}`}><Info size={16}/></span><small>{p.mode==='demo'?`${w.preview} · `:''}{w.count.replace('{n}',String(snapshot.history.length))} · {w.sessionOnly}</small></div>
 {error&&<p role="alert">{w.error}</p>}
 </div>;
}

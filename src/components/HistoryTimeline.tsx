import {useMemo,useState} from 'react';
import {Activity,AppWindow,Globe2,Info,RotateCcw} from 'lucide-react';
import type {Connection,Snapshot} from '../core/types';
import {historyTimeline,inHistoryRange,timelineWords} from '../core/history-timeline';
import {HistoryRetention} from './HistoryRetention';
import {Connections} from './Connections';
import type {T} from './ui';
import './HistoryTimeline.css';
export function HistoryTimeline({snapshot,search,t,select}:{snapshot:Snapshot;search:string;t:T;select:(row:Connection)=>void}){
 const locale=snapshot.document.preferences.locale,w=timelineWords[locale];
 const [range,setRange]=useState<{start:number;end:number}|null>(null);
 const rows=useMemo(()=>snapshot.history.filter(c=>`${c.app} ${c.destination}`.toLowerCase().includes(search.toLowerCase())),[snapshot.history,search]);
 const buckets=useMemo(()=>historyTimeline(rows),[rows]);
 const filtered=rows.filter(row=>inHistoryRange(row,range));
 const max=Math.max(1,...buckets.map(b=>b.count));
 const time=(value:number)=>new Intl.DateTimeFormat(locale,{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}).format(value*1000);
 return <><section className="surface history-controls"><HistoryRetention snapshot={snapshot} compact/></section>
 <section className="surface history-timeline" aria-label={w.title}>
 <header><h2><Activity size={20}/>{w.title}</h2><span>{snapshot.document.preferences.mode==='demo'&&w.demo}</span><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={17}/></span></header>
 <div className="ht-metrics">{[{Icon:Activity,count:filtered.length,label:w.records},{Icon:AppWindow,count:new Set(filtered.map(c=>c.appKey||`${c.app}:${c.pid}`)).size,label:w.apps},{Icon:Globe2,count:new Set(filtered.map(c=>c.destination)).size,label:w.destinations}].map(({Icon,count,label})=><div key={label}><Icon size={18}/><strong>{count.toLocaleString(locale)}</strong><span>{label}</span></div>)}</div>
 {buckets.length?<><div className="ht-chart" role="group" aria-label={w.title}>{buckets.map(bucket=>{
 const selected=range?.start===bucket.start&&range.end===bucket.end;
 const label=`${time(bucket.start)} – ${time(bucket.end)} · ${bucket.count} ${w.records}`;
 return <button key={bucket.start} className={`ht-bar ${selected?'selected':''}`} aria-label={label} title={label} aria-pressed={selected} onClick={()=>setRange(selected?null:{start:bucket.start,end:bucket.end})}><span className="ht-count">{bucket.count}</span><span className="ht-track"><i style={{height:`${bucket.count/max*100}%`}}/></span><span className="ht-tick">{new Intl.DateTimeFormat(locale,{hour:'2-digit',minute:'2-digit'}).format(bucket.start*1000)}</span></button>;
 })}</div><div className="ht-axis"><span>{time(buckets[0].start)}</span><span>{time(buckets[buckets.length-1].end)}</span></div></>:<p>{w.empty}</p>}
 <footer><span role="status">{range?`${w.range}: ${time(range.start)} – ${time(range.end)}`:w.all}</span><button className="text-button" disabled={!range} onClick={()=>setRange(null)}><RotateCcw size={14}/>{w.all}</button></footer>
 </section><Connections connections={filtered} t={t} locale={locale} activeIds={snapshot.connections.map(c=>c.id)} select={select} title={t('history')}/></>;
}

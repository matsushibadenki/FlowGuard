import {useState} from 'react';
import {Info,Search} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {collectFlowNames,native,setFlowNames} from '../core/store';
import {flowNamesFor,flowNameWords} from '../core/flow-names';
export function FlowNameEvidence({snapshot,ids}:{snapshot:Snapshot;ids:string[]}){const rows=flowNamesFor(snapshot,ids),w=flowNameWords[snapshot.document.preferences.locale];return <>{rows.slice(0,8).map((r,i)=><span className="destination-name-evidence" key={`${r.value}-${i}`}><strong>{r.value}</strong><small>{w.source} · {w.unverified}</small><small>{w.time}: {new Date(r.observedAt*1000).toLocaleString(snapshot.document.preferences.locale)}</small></span>)}</>;}
export function FlowNamesControl({snapshot}:{snapshot:Snapshot}){
 const s=snapshot.flowNames??{enabled:false,searchEnabled:false,busy:false,operation:0,rows:[],error:null},w=flowNameWords[snapshot.document.preferences.locale];const [pending,setPending]=useState(false),[error,setError]=useState(false);
 async function work(action:()=>Promise<void>){setPending(true);setError(false);try{await action();}catch{setError(true);}finally{setPending(false);}}
 return <section className="destination-names-control"><div className="destination-names-heading"><strong>{w.title}</strong><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span></div><label><input type="checkbox" checked={s.enabled} disabled={!native||pending||(!s.enabled&&(snapshot.document.preferences.mode!=='live'||!snapshot.extension.signed))} onChange={e=>void work(()=>setFlowNames(e.target.checked,s.searchEnabled,s.operation))}/>{w.enable}</label><label><input type="checkbox" checked={s.searchEnabled} disabled={!native||pending||!s.enabled} onChange={e=>void work(()=>setFlowNames(s.enabled,e.target.checked,s.operation))}/>{w.search}</label><button className="secondary-button" disabled={!native||!s.enabled||s.busy||pending||snapshot.updates.stage==='installing'} onClick={()=>void work(()=>collectFlowNames(s.operation))}><Search size={16}/>{w.collect}</button>{(error||s.error)&&<small role="alert">{w.error}</small>}</section>;
}

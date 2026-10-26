import {useState} from 'react';
import {ExternalLink,PanelLeft,PanelRight,PanelBottom,MousePointer2} from 'lucide-react';
import {positionPortDetails,selectPort} from '../core/store';
import {portPanelWords,portPanelStages,type PortPanel} from '../core/port-panel';
import {PortTopology,PortRoute} from './PortTopology';
import type {Snapshot} from '../core/types';
import './PortDetailPanel.css';
export function PortDetailPanel({snapshot,native,standalone=false,onPosition=positionPortDetails}:{snapshot:Snapshot;native:boolean;standalone?:boolean;onPosition?:(panel:PortPanel)=>Promise<unknown>}){
 const job=snapshot.ports,locale=snapshot.document.preferences.locale,w=portPanelWords[locale],panel=job.panel??'bottom';
 const [pending,setPending]=useState(false),[error,setError]=useState(false);
 async function position(value:PortPanel){setPending(true);setError(false);try{await onPosition(value);}catch{setError(true);}finally{setPending(false);}}
 const selected=job.rows.some(r=>r.port===job.selectedPort);
 return <aside className={`port-detail-panel port-topology ${standalone?'standalone':''}`} aria-label={w.title}><header><h3>{w.title}</h3><div role="group" aria-label={w.title}>{(['left','bottom','right','floating'] as const).map((target,i)=>{const Icon=[PanelLeft,PanelBottom,PanelRight,ExternalLink][i];return <button type="button" className="icon-button" key={target} disabled={!native||pending} aria-pressed={panel===target} title={w[target]} aria-label={w[target]} onClick={()=>void position(target)}><Icon size={16}/></button>;})}</div></header>{error&&<p role="alert">{w.error}</p>}{!standalone&&panel==='floating'?<div className="port-panel-empty"><ExternalLink size={24}/><span>{w.detached}</span><button type="button" className="icon-button" disabled={!native||pending} aria-label={w.floating} title={w.floating} onClick={()=>void position('floating')}><ExternalLink size={18}/></button></div>:<>{standalone&&<PortTopology job={job} locale={locale} disabled={!native} showDetail={false} onSelect={port=>void selectPort(job.operation,port).catch(()=>setError(true))}/>}<div className="port-panel-context"><code>{job.target||'—'}</code><small>{job.interface} · #{job.operation} · LAN #{job.generation}</small>{job.stage&&<span className="port-panel-stage">{portPanelStages[locale][job.stage as 'running']??portPanelStages[locale].failed}</span>}</div>{job.stage&&job.generation!==snapshot.inventory.generation&&<p role="status">{w.stale}</p>}{selected?<PortRoute job={job} locale={locale}/>:<div className="port-panel-empty"><MousePointer2 size={26}/><span>{w.empty}</span></div>}</>}</aside>;
}

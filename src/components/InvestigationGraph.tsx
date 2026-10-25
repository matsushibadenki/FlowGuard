// パス: src/components/InvestigationGraph.tsx
import {Fragment,useState} from 'react';
import {Network,Plug,Server,ScanSearch,Info,ArrowUpRight,LoaderCircle,Clock3,Link2,Unlink,ShieldQuestion} from 'lucide-react';
import type {DeviceChange,Snapshot} from '../core/types';
import {investigationGraph,investigationWords,type InvestigationNode,type Segment} from '../core/investigation-graph';
import {diagnosticAction,dispatch,openLanPorts} from '../core/store';
import './InvestigationGraph.css';
const nodes=[['device',Network],['ports',Plug],['services',Server],['diagnostics',ScanSearch]] as const;
export function InvestigationGraph({snapshot,row,native}:{snapshot:Snapshot;row:DeviceChange;native:boolean}) {
  const [selected,setSelected]=useState<InvestigationNode>('ports'),[busy,setBusy]=useState(false),[error,setError]=useState(false);
  const state=investigationGraph(snapshot,row),w=investigationWords[snapshot.document.preferences.locale];
  const label=(key:string)=>w[key as 'open']??key;
  const linked={device:state.current,ports:state.hasPorts,services:state.hasServices,diagnostics:!!state.report};
  const stage=(value:string)=>label(value==='failed'?'interrupted':value);
  const status={device:state.current?w.record:w.stale,ports:state.hasPorts?stage(snapshot.ports.stage):w.none,services:state.hasServices?stage(snapshot.services.stage):w.none,diagnostics:state.report?(state.reportLoaded?w.record:w.load):w.none};
  const values={device:state.current?'1':'—',ports:state.hasPorts?String(state.open):'—',services:state.hasServices?String(state.serviceConfirmed):'—',diagnostics:state.report?(state.reportLoaded?String(state.report.findings.length):'—'):'—'};
  const captions={device:row.device.family,ports:w.open,services:w.confirmed,diagnostics:state.reportLoaded?w.items:`${state.reports} ${w.reports}`};
  const detail={device:w.deviceHelp,ports:w.portsHelp,services:w.servicesHelp,diagnostics:w.diagnosticsHelp};
  const segments=selected==='ports'?state.portSegments:selected==='services'?state.serviceSegments:selected==='diagnostics'?state.diagnosticSegments:[];
  const available=linked[selected]&&(selected!=='diagnostics'||state.reportLoaded);
  const operation=selected==='ports'?snapshot.ports.operation:selected==='services'?snapshot.services.operation:state.report?.id;
  async function action(open=false) {
    setBusy(true);setError(false);
    try {
      if(selected==='ports')await openLanPorts(row.device.id,snapshot.inventory.generation);
      if(selected==='services')await dispatch({type:'setView',view:'servers'});
      if(selected==='diagnostics'&&state.report){await diagnosticAction('select',state.report.id);if(open)await dispatch({type:'setView',view:'diagnostics'});}
    }catch{setError(true);}finally{setBusy(false);}
  }
  return <section className="investigation-graph" aria-label={w.title}>
    <header className="iv-header"><span className="iv-target-icon"><Network size={30}/></span><div><small>{w.title}</small><h3>{row.device.address}</h3><span>{row.device.interface} · {w.scope} #{snapshot.inventory.generation}</span></div><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={18}/></span></header>
    <div className="iv-chain" role="group" aria-label={w.title}>{nodes.map(([key,Icon],index)=><Fragment key={key}>
      {index>0&&<span className={`iv-link ${linked[key]?'linked':''}`} aria-hidden="true"/>}
      <button className={`iv-node ${linked[key]?'linked':'missing'} ${selected===key?'selected':''}`} aria-pressed={selected===key} onClick={()=>{setSelected(key);setError(false);}}>
        <span className="iv-node-heading"><Icon size={25}/><strong>{w[key]}</strong></span><span className="iv-number">{values[key]}</span><span className="iv-caption">{captions[key]}</span>
        <span className="iv-node-status">{linked[key]?<Link2 size={13}/>:<Unlink size={13}/>} {status[key]}</span>
      </button>
    </Fragment>)}</div>
    <div className="iv-detail" key={`${row.device.id}:${selected}`}>
      <div className="iv-detail-title"><strong>{w[selected]}</strong><span className="info-tip" tabIndex={0} title={detail[selected]} aria-label={detail[selected]}><Info size={16}/></span><span>{w.select}</span></div>
      {selected==='device'?<div className="iv-device-evidence"><Network size={38}/><div><strong>{row.device.address}</strong><span>{row.device.interface} · {row.device.source.toUpperCase()}</span><span><Clock3 size={14}/>{new Date(row.device.observedAt*1000).toLocaleString(snapshot.document.preferences.locale)}</span></div></div>:<>
        <Distribution segments={segments} available={available} locale={snapshot.document.preferences.locale}/>
        {selected==='diagnostics'&&state.reportLoaded&&<div className="iv-severity"><span><ShieldQuestion size={17}/>{w.warning} <b>{state.warning}</b></span><span><Info size={17}/>{w.info} <b>{state.info}</b></span></div>}
      </>}
      <div className="iv-actions">{linked[selected]&&selected!=='device'&&<small>{w.operation} #{operation}{selected==='diagnostics'&&state.report&&<> · {new Date(state.report.createdAt*1000).toLocaleString(snapshot.document.preferences.locale)}</>}</small>}
        {selected==='diagnostics'&&state.report&&!state.reportLoaded&&<button className="secondary-button" disabled={!native||busy} onClick={()=>void action()}>{busy?<LoaderCircle className="iv-working" size={16}/>:<ScanSearch size={16}/>} {busy?w.loading:w.load}</button>}
        {selected!=='device'&&<button className="secondary-button" disabled={!native||busy||!linked[selected]} onClick={()=>void action(true)}><ArrowUpRight size={17}/>{w.view}</button>}
      </div>
      {error&&<p className="iv-error" role="alert">{w.failed}</p>}
    </div>
  </section>;
}
function Distribution({segments,available,locale}:{segments:Segment[];available:boolean;locale:Snapshot['document']['preferences']['locale']}) {
  const w=investigationWords[locale],total=segments.reduce((sum,s)=>sum+s.count,0),circumference=2*Math.PI*44;
  let offset=0;
  return <div className={`iv-distribution ${available?'':'unavailable'}`}>
    <div className="iv-ring" role="img" aria-label={available?segments.map(s=>`${w[s.key as 'open']}: ${s.count}`).join(', '):w.none}>
      <svg viewBox="0 0 112 112" aria-hidden="true"><circle className="iv-track" cx="56" cy="56" r="44"/>{available&&segments.map(s=>{const length=total?s.count/total*circumference:0;const start=offset;offset+=length;return <circle key={s.key} className={`iv-arc tone-${s.tone}`} cx="56" cy="56" r="44" strokeDasharray={`${length} ${circumference-length}`} strokeDashoffset={-start}/>;})}</svg>
      <div><strong>{available?total:'—'}</strong><small>{w.items}</small></div>
    </div>
    <div className="iv-bars">{available?segments.map(s=><div className={`iv-bar-row tone-${s.tone}`} key={s.key}><span>{w[s.key as 'open']}</span><strong>{s.count}</strong><div className="iv-bar-track"><i style={{width:`${total?s.count/total*100:0}%`}}/></div></div>):<p><Unlink size={22}/>{w.empty}</p>}</div>
  </div>;
}

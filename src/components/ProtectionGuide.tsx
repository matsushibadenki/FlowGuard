import {lazy,Suspense,useState} from 'react';
import {BookOpen,Info} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {dispatch} from '../core/store';
import {needsProtectionGuide,protectionGuideWords} from '../core/protection-guide';
const ControlGraph=lazy(()=>import('./ControlGraph').then(m=>({default:m.ControlGraph})));
export function ProtectionGuide({snapshot,settings=false,expanded=false,onClose}:{snapshot:Snapshot;settings?:boolean;expanded?:boolean;onClose?:()=>void}){
 const [replay,setReplay]=useState(false),[busy,setBusy]=useState(false),[failed,setFailed]=useState(false);
 const unseen=needsProtectionGuide(snapshot.document.preferences),w=protectionGuideWords[snapshot.document.preferences.locale];
 if((settings&&unseen)||(!settings&&!unseen&&!expanded))return null;
 if(settings&&!replay)return <section className="surface settings-section"><button className="secondary-button" onClick={()=>setReplay(true)}><BookOpen size={18}/>{w.reopen}</button></section>;
 async function acknowledge(){setBusy(true);setFailed(false);try{await dispatch({type:'acknowledgeProtectionGuide'});onClose?.();}catch{setFailed(true);}finally{setBusy(false);}}
 const installing=snapshot.updates.stage==='installing';
 const review=settings||!unseen;
 return <section id={settings?undefined:"main-protection-guide"} className="protection-guide surface" aria-label={w.title}>
  <div className="protection-guide-heading"><BookOpen size={20}/><h2>{w.title}</h2><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span></div>
  <Suspense fallback={<p role="status">{w.loading}</p>}><ControlGraph snapshot={snapshot}/></Suspense>
  <div className="protection-guide-footer">{snapshot.document.preferences.mode==='demo'&&<small>{w.demo}</small>}<button className="secondary-button" disabled={!review&&(busy||installing)} title={w.help} onClick={()=>review?(settings?setReplay(false):onClose?.()):void acknowledge()}>{review?w.close:busy?w.saving:w.ack}</button></div>
  {failed&&<p role="alert">{w.saveFailed}</p>}{!review&&installing&&<small role="status">{w.blocked}</small>}
 </section>;
}

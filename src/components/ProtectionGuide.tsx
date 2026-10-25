import {useState} from 'react';
import {BookOpen,Eye,FileCheck2,Settings2,ListChecks,Info} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {dispatch} from '../core/store';
import {needsProtectionGuide,protectionGuideWords} from '../core/protection-guide';
const icons=[Eye,FileCheck2,Settings2,ListChecks];
export function ProtectionGuide({snapshot,settings=false}:{snapshot:Snapshot;settings?:boolean}){
 const [replay,setReplay]=useState(false),[busy,setBusy]=useState(false),[failed,setFailed]=useState(false);
 const unseen=needsProtectionGuide(snapshot.document.preferences),w=protectionGuideWords[snapshot.document.preferences.locale];
 if((settings&&unseen)||(!settings&&!unseen))return null;
 if(settings&&!replay)return <section className="surface settings-section"><button className="secondary-button" onClick={()=>setReplay(true)}><BookOpen size={18}/>{w.reopen}</button></section>;
 async function acknowledge(){setBusy(true);setFailed(false);try{await dispatch({type:'acknowledgeProtectionGuide'});}catch{setFailed(true);}finally{setBusy(false);}}
 const installing=snapshot.updates.stage==='installing';
 return <section className="protection-guide surface" aria-label={w.title}>
  <div className="protection-guide-heading"><BookOpen size={20}/><h2>{w.title}</h2><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span></div>
  <p className="protection-guide-intro">{w.intro}</p>
  <ol>{w.steps.map((step,i)=>{const Icon=icons[i];return <li key={i}><div><span className="protection-guide-number">{i+1}</span><Icon size={17}/><strong>{step.title}</strong></div><p>{(step.text.match(/[^。.!?]+[。.!?]?/g)??[step.text]).map((sentence,index)=><span className="protection-guide-sentence" key={index}>{sentence.trim()}</span>)}</p></li>;})}</ol>
  <div className="protection-guide-footer">{snapshot.document.preferences.mode==='demo'&&<small>{w.demo}</small>}<button className="secondary-button" disabled={!settings&&(busy||installing)} title={w.help} onClick={()=>settings?setReplay(false):void acknowledge()}>{settings?w.close:busy?w.saving:w.ack}</button></div>
  {failed&&<p role="alert">{w.saveFailed}</p>}{!settings&&installing&&<small role="status">{w.blocked}</small>}
 </section>;
}

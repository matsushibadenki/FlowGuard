import {useRef,useState} from 'react';
import {Info,Search,Trash2} from 'lucide-react';
import type {Connection,DestinationNameRow,Locale,Snapshot} from '../core/types';
import {clearDestinationNames,lookupDestinationName,native,setDestinationNames} from '../core/store';
import {destinationName,destinationNameWords,emptyDestinationNames,nameLookupEligible} from '../core/destination-names';
export function DestinationNameEvidence({row,locale}:{row:DestinationNameRow|undefined;locale:Locale}){
 if(!row)return null;const w=destinationNameWords[locale];
 const outcome=['resolved','no_name','invalid_name','failed','timed_out'].includes(row.status)?w[row.status as 'resolved']:w.failed;
 return <span className="destination-name-evidence"><strong>{row.value??outcome}</strong><span>{w.unverified}{row.stale?` · ${w.stale}`:''}</span><span>{w.source}: {w.resolver}</span><time dateTime={new Date(row.observedAt*1000).toISOString()}>{w.time}: {new Intl.DateTimeFormat(locale,{dateStyle:'short',timeStyle:'medium'}).format(row.observedAt*1000)}</time></span>;
}
export function DestinationNamesControl({snapshot,connection}:{snapshot:Snapshot;connection?:Connection}){
 const [pending,setPending]=useState(false),[failed,setFailed]=useState(false);const request=useRef(0);
 const s=snapshot.destinationNames??emptyDestinationNames,w=destinationNameWords[snapshot.document.preferences.locale],live=native&&snapshot.document.preferences.mode==='live';
 const installing=snapshot.updates.stage==='installing';
 async function run(work:()=>Promise<void>){const token=++request.current;setPending(true);setFailed(false);try{await work();}catch{if(token===request.current)setFailed(true);}finally{if(token===request.current)setPending(false);}}
 return <section className="destination-names-control"><div className="destination-names-heading"><strong>{w.title}</strong><span className="info-tip" title={w.help} aria-label={w.help} tabIndex={0}><Info size={15}/></span></div>
 <label><input type="checkbox" checked={s.enabled} disabled={!native||(!s.enabled&&(!live||installing||pending))} onChange={e=>void run(()=>setDestinationNames(e.target.checked,s.operation))}/>{w.enable}</label>
 {!live&&<small>{w.preview}</small>}
 <small>{w.disclosure}</small>
 {connection&&<><button className="secondary-button" disabled={!live||!s.enabled||s.busy||pending||installing||s.cooldownSeconds>0||!nameLookupEligible(connection.destination)} onClick={()=>void run(()=>lookupDestinationName(connection.id,connection.destination,s.operation))}><Search size={15}/>{w.lookup}</button>
 {!nameLookupEligible(connection.destination)&&<small>{w.unsupported}</small>}<DestinationNameEvidence row={destinationName(snapshot,connection.destination)} locale={snapshot.document.preferences.locale}/></>}
 <div className="destination-names-actions"><button className="text-button" disabled={!native||pending||(!s.rows.length&&!s.busy)} onClick={()=>void run(()=>clearDestinationNames(s.operation))}><Trash2 size={14}/>{w.clear}</button>{s.busy?<small role="status">{s.cancelling?w.cancelling:w.busy}{s.activeAddress?` · ${s.activeAddress}`:''}</small>:s.cooldownSeconds>0&&<small role="status">{w.cooldown.replace('{n}',String(s.cooldownSeconds))}</small>}</div>
 {failed&&<small role="alert">{w.error}</small>}
 </section>;
}

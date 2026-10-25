import {useState} from 'react';
import {Info,Trash2} from 'lucide-react';
import {dispatch,clearUpdateCache} from '../core/store';
import {updateExtrasWords} from '../core/update-extras';
import type {Snapshot} from '../core/types';
export function UpdateExtras({snapshot,desktop}:{snapshot:Snapshot;desktop:boolean}){
 const w=updateExtrasWords[snapshot.document.preferences.locale],u=snapshot.updates;const [pending,setPending]=useState(false),[error,setError]=useState(false),[cleared,setCleared]=useState(false);const busy=['checking','downloading','installing'].includes(u.stage);
 async function work(run:()=>Promise<unknown>){setPending(true);setError(false);try{await run();}catch{setError(true);}finally{setPending(false);}}
 return <div className="update-extras"><div className="setting-row"><label htmlFor="update-channel">{w.channel}</label><select id="update-channel" disabled={!desktop||pending||busy||u.stage==='ready'} value={snapshot.document.preferences.updateChannel??'stable'} onChange={e=>void work(()=>dispatch({type:'setUpdateChannel',channel:e.target.value as 'stable'|'beta'}))}><option value="stable">{w.stable}</option><option value="beta">{w.beta}</option></select><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span></div><button className="text-button" disabled={!desktop||busy||pending} onClick={()=>void work(async()=>{await clearUpdateCache();setCleared(true);})}><Trash2 size={16}/>{w.clear}</button>{cleared&&<small role="status">{w.cleared}</small>}{error&&<small role="alert">{w.failed}</small>}{u.historyPersistent===false&&<small role="alert">{w.persistent}</small>}{!!u.history?.length&&<details><summary>{w.history}</summary><ol>{u.history.map((e,i)=><li key={i}>{new Date(e.at*1000).toLocaleString(snapshot.document.preferences.locale)} · {e.from} → {e.to??'—'} · {e.status==='launched_version'?w.launch:e.status==='installing'?w.install:w[e.status as 'ready']??w.failed}</li>)}</ol></details>}</div>;
}

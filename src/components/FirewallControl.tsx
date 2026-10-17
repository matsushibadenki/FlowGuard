import {useState} from 'react';
import {Info,Shield,RefreshCw} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {controlFirewall,native} from '../core/store';
import {emptyFirewallControl,firewallWords,firewallWriteAllowed,type FirewallAction} from '../core/firewall-control';
export function FirewallControl({snapshot,onError}:{snapshot:Snapshot;onError:(e:unknown)=>void}){
 const s=snapshot.firewallControl??emptyFirewallControl,w=firewallWords[snapshot.document.preferences.locale];
 const [ack,setAck]=useState(false);
 const available=native&&!s.busy&&snapshot.updates.stage!=='installing';
 async function run(action:FirewallAction){const acknowledged=ack;setAck(false);try{await controlFirewall(action,s.enabled,acknowledged);}catch(e){onError(e);}}
 const service=s.service&&s.service in w?w[s.service as 'enabled']:null;
 return <section className="surface settings-section firewall-control"><div className="settings-heading"><Shield size={22}/><h2>{w.title}</h2><span className="info-tip" tabIndex={0} aria-label={w.help} title={w.help}><Info size={16}/></span></div>
 <strong role="status" aria-live="polite">{s.busy?w.busy:s.enabled===null?w.unknown:s.enabled?w.on:w.off}</strong>
 {service&&<p>{service}</p>}{!native&&<p>{w.preview}</p>}
 {s.error&&<p role="alert">{s.error==='authorization_cancelled'?w.cancelled:s.error==='state_changed'?w.changed:w.error}</p>}
 <div className="button-row"><button className="text-button" disabled={!available} onClick={()=>void run('status')}><RefreshCw size={16}/>{w.refresh}</button></div>
 <label className="firewall-ack"><input type="checkbox" checked={ack} disabled={!available} onChange={e=>setAck(e.target.checked)}/>{w.ack}</label>
 <div className="button-row">{s.service==='requires_approval'&&<button className="secondary-button" disabled={!available||!ack} onClick={()=>void run('approval_settings')}>{w.approval}</button>}<button className="secondary-button" disabled={!available||!ack||s.service!=='not_registered'} onClick={()=>void run('register')}>{w.register}</button><button className="secondary-button" disabled={!available||!ack||!['enabled','requires_approval'].includes(s.service)} onClick={()=>void run('unregister')}>{w.unregister}</button></div>
 <div className="button-row"><button className="secondary-button" disabled={!available||!firewallWriteAllowed(s,ack,native)||s.enabled===true} onClick={()=>void run('enable')}>{w.enable}</button><button className="secondary-button" disabled={!available||!firewallWriteAllowed(s,ack,native)||s.enabled===false} onClick={()=>void run('disable')}>{w.disable}</button></div>
 </section>;
}

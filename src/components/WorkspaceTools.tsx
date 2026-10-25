import {lazy,Suspense} from 'react';
const DiagnosticChecks=lazy(()=>import('./DiagnosticChecks').then(m=>({default:m.DiagnosticChecks})));
import {SyslogReceiver} from './SyslogReceiver';
const ServiceChecks=lazy(()=>import('./ServiceChecks').then(m=>({default:m.ServiceChecks})));
import {LanDevices} from './LanDevices';
import type {Snapshot,View} from '../core/types';
import type {T} from './ui';
export function WorkspaceTools({snapshot,view,t,native}:{snapshot:Snapshot;view:View;t:T;native:boolean}) {
 const network=snapshot.network;
 if(view==='diagnostics')return <div className="tools-workspace"><Suspense fallback={<p>{t('loading')}</p>}><DiagnosticChecks snapshot={snapshot} native={native}/></Suspense><SyslogReceiver snapshot={snapshot} native={native}/></div>;
 if(view==='servers')return <div className="tools-workspace"><Suspense fallback={<p>{t('loading')}</p>}><ServiceChecks snapshot={snapshot} native={native}/></Suspense></div>;
 return <div className="tools-workspace"><section className="surface tools-summary"><span className="tool-stage">{t(view==='network'?(native?'lanTools':'lanDesktopOnly'):'plannedFeature')}</span><h2>{t(view==='network'?'network':'diagnostics')}</h2><p>{t(view==='network'?(native?'lanSession':'interfacePreview'):'diagnosticPlan')}</p></section>
 {view==='network'&&<LanDevices snapshot={snapshot} t={t} native={native}/>}
 {view==='network'&&<section className="surface"><div className="section-head"><h2>{t('interfaces')}</h2><span>{network.addresses.length} {t('addressCount')}</span></div>{network.error&&<p className="tool-message" role="alert">{t('interfaceError')}</p>}{!network.addresses.length?<p className="tool-message">{t(native?'noInterfaces':'interfacePreview')}</p>:<div className="table-scroll tool-table"><table><thead><tr>{['interfaceName','address','protocol','networkMask','status'].map(key=><th key={key}>{t(key as 'address')}</th>)}</tr></thead><tbody>{network.addresses.map(row=><tr key={`${row.interface}-${row.address}`}><td>{row.interface}</td><td className="endpoint">{row.address}</td><td>{row.family}</td><td>{row.netmask||t('unknown')}</td><td>{t(row.up?'interfaceUp':'interfaceDown')}</td></tr>)}</tbody></table></div>}</section>}
 <section className="surface tools-summary"><h2>{t('plannedWork')}</h2><ul>{(view==='network'?['portPlan']:['tlsPlan','configurationPlan','advisoryPlan','reportPlan']).map(key=><li key={key}><span className="planned-label">{t('plannedFeature')}</span>{t(key as 'devicePlan')}</li>)}</ul></section></div>;
}

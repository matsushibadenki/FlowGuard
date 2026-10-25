import type {Connection,Snapshot,Locale} from '../core/types';
import {AppIcon,Empty,type T} from './ui';
import {Connections} from './Connections';
export function Applications({snapshot,connections,t,locale,select}:{snapshot:Snapshot;connections:Connection[];t:T;locale:Locale;select:(c:Connection)=>void}){
 const visible=new Map(connections.map(c=>[c.id,c]));
 const groups=snapshot.applications.map(group=>({...group,connections:group.connectionIds.flatMap(id=>{const c=visible.get(id);return c?[c]:[];})})).filter(group=>group.connections.length>0);
 return <section className="applications-view surface"><div className="section-head"><h2>{t('appConnections')}</h2></div><p className="application-help">{t('groupHelp')}</p>{groups.length===0?<Empty t={t}/>:groups.map(group=><details className="application-group" key={group.id}><summary><AppIcon app={group.app} appKey={group.appKey}/><span className="application-summary"><strong>{group.app}</strong><small title={group.appKey}>{group.appKey||t('noIdentity')}</small><span>{group.connections.length} {t('count')} · {new Set(group.connections.map(c=>c.pid)).size} {t('processCount')} · {new Set(group.connections.map(c=>`${c.destination}:${c.port}`)).size} {t('destinationCount')}</span></span></summary><Connections connections={group.connections} t={t} locale={locale} select={select}/></details>)}</section>;
}

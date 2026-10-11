import {Network,LayoutGrid,FileSliders,Clock3,Settings2,Monitor,Info} from 'lucide-react';
import type {Snapshot,View} from '../core/types';
import {Brand,type T} from './ui';
const items=[['map',Network],['apps',LayoutGrid],['rules',FileSliders],['history',Clock3],['settings',Settings2]] as const;
export function Sidebar({snapshot,t,navigate}:{snapshot:Snapshot;t:T;navigate:(view:View)=>void}){
 const enabled=snapshot.firewall.enabled;
 return <aside className="sidebar"><div className="wordmark"><Brand/><div><strong>FlowGuard</strong><p>{t('tagline')}</p></div></div>
 <nav aria-label="FlowGuard">{items.map(([view,Icon])=><button key={view} aria-label={t(view)} aria-current={snapshot.document.preferences.view===view?'page':undefined} className={snapshot.document.preferences.view===view?'active':''} onClick={()=>navigate(view)}><Icon size={21} strokeWidth={1.7}/><span>{t(view)}</span></button>)}</nav>
 <div className="sidebar-bottom"><div className="system-title"><Monitor size={21}/><div><strong>macOS</strong><span>{t('observationOnly')}</span></div><Info size={16}/></div><p>{t('macFirewall')}<br/><strong className={enabled===true?'text-allow':enabled===false?'text-block':''}>{t(enabled===null?'unknown':enabled?'enabled':'disabled')}</strong></p><button className="text-button" onClick={()=>navigate('settings')}>{t('settings')} <span>↗</span></button></div>
 <span className="version">FlowGuard 0.1.0</span></aside>;
}

import {useState} from 'react';
import {Globe,Network,LayoutGrid,FileSliders,Clock3,Settings2,Monitor,Info,Server,ShieldCheck,Waypoints} from 'lucide-react';
import type {Snapshot,View,Locale} from '../core/types';
import {dispatch} from '../core/store';
import {extensionLabel} from '../core/enforcement';
import {Brand,type T} from './ui';
const groups=[{label:'firewallCategory',items:[['map',Network],['apps',LayoutGrid],['rules',FileSliders],['history',Clock3]]},{label:'networkCategory',items:[['network',Waypoints],['whois',Globe]]},{label:'serverCategory',items:[['servers',Server]]},{label:'securityCategory',items:[['diagnostics',ShieldCheck]]},{label:'commonCategory',items:[['settings',Settings2]]}] as const;
export function Sidebar({snapshot,t,navigate,onError}:{snapshot:Snapshot;t:T;navigate:(view:View)=>void;onError:(error:unknown)=>void}){
 const [menuOpen,setMenuOpen]=useState(true);
 const enabled=snapshot.firewall.enabled;
 return <aside className="sidebar"><div className="wordmark"><Brand/><div><strong>FlowGuard</strong><p>{t('tagline')}</p></div></div>
 <div className={`navigation-menu ${menuOpen?'menu-open':''}`}><button className="navigation-toggle" aria-expanded={menuOpen} aria-controls="workspace-navigation" onClick={()=>setMenuOpen(!menuOpen)}>{t('navigation')}</button><nav id="workspace-navigation" aria-label="FlowGuard">{groups.map(group=><div className="nav-group" key={group.label}><h2>{t(group.label)}</h2>{group.items.map(([view,Icon])=><button key={view} aria-label={t(view)} aria-current={snapshot.document.preferences.view===view?'page':undefined} className={snapshot.document.preferences.view===view?'active':''} onClick={()=>navigate(view)}><Icon size={20} strokeWidth={1.7}/><span>{t(view)}</span></button>)}</div>)}</nav></div>
 <div className="sidebar-bottom"><div className="system-title"><Monitor size={21}/><div><strong>macOS</strong><span>{t(snapshot.document.preferences.mode==='live'?extensionLabel(snapshot.extension.stage):'observationOnly')}</span></div><Info size={16}/></div><p>{t('macFirewall')}<br/><strong className={enabled===true?'text-allow':enabled===false?'text-block':''}>{t(enabled===null?'unknown':enabled?'enabled':'disabled')}</strong></p><button className="text-button" onClick={()=>navigate('settings')}>{t('settings')} <span>↗</span></button></div>
 <div className="sidebar-footer"><label className="language-picker"><Globe size={17}/><span className="sr-only">{t('language')}</span><select aria-label={t('language')} value={snapshot.document.preferences.locale} onChange={e=>void dispatch({type:'setLocale',locale:e.target.value as Locale}).catch(onError)}><option value="ja">日本語</option><option value="en">English</option><option value="zh-CN">简体中文</option></select></label><span className="version">FlowGuard {snapshot.updates.currentVersion}</span></div></aside>;
}

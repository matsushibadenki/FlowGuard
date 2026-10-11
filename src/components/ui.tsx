import {policyNotice} from '../core/enforcement';
import {Compass,Mail,Box,Terminal,AppWindow,Shield,Check,Minus,HelpCircle,ArrowUpRight,ArrowDownLeft,ArrowLeftRight,Info} from 'lucide-react';
import type {Action} from '../core/types';
import type {TextKey} from '../core/i18n';
export type T=(key:TextKey)=>string;
export function AppIcon({app,small=false}:{app:string;small?:boolean}){
 const Icon=app.toLowerCase().includes('safari')?Compass:app.toLowerCase().includes('mail')?Mail:app.toLowerCase().includes('dropbox')?Box:app.toLowerCase().includes('terminal')?Terminal:AppWindow;
 const tone=Icon===Terminal?'terminal':Icon===Box?'box':Icon===Mail?'mail':Icon===Compass?'safari':'generic';
 return <span className={`app-icon ${tone} ${small?'small':''}`}><Icon aria-hidden="true" size={small?17:25} strokeWidth={1.7}/></span>;
}
export function Brand({large=false}:{large?:boolean}) {return <span className={`brand-shield ${large?'large':''}`}><Shield aria-hidden="true" size={large?46:34} strokeWidth={1.8}/><Check className="shield-check" size={large?19:14} strokeWidth={2.7}/></span>;}
export function ActionIcon({action,size=17}:{action:Action;size?:number}){const Icon=action==='allow'?Check:action==='block'?Minus:HelpCircle;return <Icon size={size} aria-hidden="true"/>;}
export function DirectionIcon({direction}:{direction:string}){const Icon=direction==='outbound'?ArrowUpRight:direction==='inbound'?ArrowDownLeft:ArrowLeftRight;return <Icon size={16} aria-hidden="true"/>;}
export function Notice({t,compact=false,snapshot}:{t:T;compact?:boolean;snapshot?:import('../core/types').Snapshot}){return <div className={`notice ${compact?'compact':''}`}><Info size={18} aria-hidden="true"/><div><strong>{t(snapshot?policyNotice(snapshot.extension,snapshot.document.preferences.mode):'noEnforcement')}</strong>{!compact&&<p>{t(snapshot&&snapshot.extension.signed?'appliedPolicyHelp':'signatureHelp')}</p>}</div></div>;}
export function Empty({t,rules=false}:{t:T;rules?:boolean}){return <div className="empty"><Shield size={32} strokeWidth={1.4}/><h3>{t(rules?'noRules':'empty')}</h3><p>{t(rules?'noRulesHelp':'emptyHelp')}</p></div>;}

import {Globe,Network,ChevronRight} from 'lucide-react';
import type {Connection,Snapshot} from '../core/types';
import {ruleFor} from '../core/rules';
import {AppIcon,Brand,Empty,Notice,type T} from './ui';
export function FlowMap({snapshot,connections,t,select}:{snapshot:Snapshot;connections:Connection[];t:T;select:(c:Connection)=>void}){
 const visible=connections.slice(0,6);const height=Math.max(420,visible.length*100+42);
 const mode=snapshot.document.preferences.mode;
 function action(c:Connection){return ruleFor(snapshot.document.rules,c,mode)?.action??'none';}
 return <section className="map-surface" aria-label={t('map')}><div className="map-head"><strong>{t('yourMac')}</strong><strong>{t('firewallRules')}</strong><strong>{t('destinations')}</strong></div>
 {visible.length===0?<Empty t={t}/>:<div className="map-canvas" style={{'--map-height':`${height}px`} as React.CSSProperties}>
 <svg className="routes" viewBox={`0 0 800 ${height}`} preserveAspectRatio="none" aria-hidden="true"><defs>{['allow','block','ask','none'].map(a=><marker key={a} id={`arrow-${a}`} markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto"><path d="M0 0 L6 3 L0 6" className={`arrow ${a}`}/></marker>)}</defs>
 {visible.map((c,i)=>{const y=72+i*100;const mid=height/2;return <g key={c.id} className={`route ${action(c)} ${snapshot.selectedId===c.id?'selected':''}`}><path d={`M 190 ${y} C 295 ${y}, 300 ${mid}, 355 ${mid}`} markerEnd={`url(#arrow-${action(c)})`}/><path d={`M 445 ${mid} C 525 ${mid}, 530 ${y}, 610 ${y}`} markerEnd={`url(#arrow-${action(c)})`}/></g>;})}</svg>
 <div className="map-nodes">{visible.map(c=><button key={c.id} className={`map-node ${snapshot.selectedId===c.id?'selected':''}`} onClick={()=>select(c)} aria-pressed={snapshot.selectedId===c.id}><AppIcon app={c.app}/><span><strong>{c.app}</strong><small>{t('observed')} · TCP</small></span><ChevronRight className="node-chevron" size={15}/></button>)}</div>
 <div className="map-center"><div className="center-orbit"><Brand large/><strong>FlowGuard</strong><small>{t('firewallRules')}</small></div></div>
 <div className="map-destinations">{visible.map(c=><button key={c.id} className={`destination-node ${snapshot.selectedId===c.id?'selected':''}`} onClick={()=>select(c)}><span className="destination-icon">{c.destination.startsWith('192.168.')||c.destination.startsWith('127.')?<Network size={24} strokeWidth={1.5}/>:<Globe size={25} strokeWidth={1.5}/>}</span><span><strong title={c.destination}>{c.destination}</strong><small>{c.port} · {c.protocol}</small></span></button>)}</div>
 </div>}
 <div className="map-footer"><div className="legend">{(['allow','block','ask','none'] as const).map(a=><span key={a}><i className={`legend-dot ${a}`}/>{t(a==='none'?'noRule':`${a}Intent`)}</span>)}</div><Notice t={t} compact/></div>
 {connections.length>6&&<p className="map-overflow">+{connections.length-6} · {t('moreConnections')}</p>}
 <details className="map-help"><summary>{t('learn')}</summary><p>{t('learnHelp')}</p></details></section>;
}

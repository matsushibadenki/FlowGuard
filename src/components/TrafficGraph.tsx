// パス: src/components/TrafficGraph.tsx
import {useEffect, useId, useLayoutEffect, useMemo, useRef, useState} from 'react';
import {Activity, AppWindow, Check, ChevronLeft, ChevronRight, CircleHelp, Globe2, Info, Laptop, Network, Minus, ShieldCheck} from 'lucide-react';
import type {AppRow, DestinationRow} from '../core/mapModel';
import type {Snapshot} from '../core/types';
import {destinationReceipt, pageWindow, trafficGraphWords} from '../core/traffic-graph';
import {destinationName, destinationNameWords} from '../core/destination-names';
import {flowNamesFor, flowNameWords} from '../core/flow-names';
import {DestinationNameEvidence} from './DestinationNames';
import {ActionIcon, AppIcon, DirectionIcon, type T} from './ui';
import './TrafficGraph.css';
const classLabels = {public:'classPublic',local:'classLocal',other:'classOther',loopback:'classLoopback'} as const;
const classIcons = {public:Globe2,local:Network,other:CircleHelp,loopback:Laptop};
const appPageSize = 4, destinationPageSize = 4;

type Props = {snapshot:Snapshot; rows:AppRow[]; t:T; selectApp:(row:AppRow)=>void; selectDestination:(row:DestinationRow)=>void};
export function TrafficGraph({snapshot, rows, t, selectApp, selectDestination}: Props) {
  const [page,setPage] = useState(0);
  const w = trafficGraphWords[snapshot.document.preferences.locale];
  const window = pageWindow(rows,page,appPageSize);
  const activeIndex = rows.findIndex(row => row.destinations.some(d => d.connectionIds.includes(snapshot.selectedId ?? '')));
  const totals = useMemo(() => rows.reduce((sum,row) => ({connections:sum.connections+row.connectionCount,destinations:sum.destinations+row.destinations.length}),{connections:0,destinations:0}),[rows]);
  return <section className="traffic-graph surface" aria-label={w.graph}>
    <header className="tg-summary">
      <div className="tg-total"><Activity size={22} aria-hidden="true"/><strong>{totals.connections.toLocaleString(snapshot.document.preferences.locale)}</strong><span>{w.connections}</span></div>
      <div className="tg-total secondary"><AppWindow size={19} aria-hidden="true"/><strong>{rows.length}</strong><span>{w.apps}</span></div>
      <div className="tg-total secondary"><Globe2 size={19} aria-hidden="true"/><strong>{totals.destinations}</strong><span>{w.destinations}</span></div>
      <span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={19}/></span>
    </header>
    <ScopeMap rows={rows} locale={snapshot.document.preferences.locale} t={t} selectDestination={(destination,index)=>{setPage(Math.floor(index/appPageSize));selectDestination(destination);}}/>
    <div className="tg-caption">{w.summary}{snapshot.document.preferences.mode==='demo'&&<span>{w.preview}</span>}</div>
    <div className="tg-legend" aria-label={w.intent}>
      <strong>{w.intent}</strong>{(['allow','block','ask','none'] as const).map(intent=><span key={intent} className={`tg-intent-${intent}`}><i/>{intent==='none'?w.noRule:t(intent)}</span>)}
      <span className="tg-line-legend"><i/>{w.lineConfirmed}</span><span className="tg-line-legend pending"><i/>{w.linePending}</span>
    </div>
    <nav className="tg-paging" aria-label={w.apps}>
      <span>{w.filtered} {window.start+1}–{window.start+window.rows.length} / {rows.length}</span>
      {activeIndex>=0&&Math.floor(activeIndex/appPageSize)!==window.current&&<button className="text-button" onClick={()=>setPage(Math.floor(activeIndex/appPageSize))}>{w.follow}</button>}
      <PageButtons current={window.current} pages={window.pages} change={setPage} locale={snapshot.document.preferences.locale}/>
    </nav>
    <div className="tg-branches">{window.rows.map(row=><AppBranch key={row.id} row={row} snapshot={snapshot} t={t} selectApp={selectApp} selectDestination={selectDestination}/>)}</div>
  </section>;
}
function ScopeMap({rows,locale,t,selectDestination}:{rows:AppRow[];locale:Snapshot['document']['preferences']['locale'];t:T;selectDestination:(row:DestinationRow,index:number)=>void}) {
  const words={ja:{title:'接続の広がり',help:'アドレス範囲に基づく模式図です。国や実際の経路、通信量は示しません。各領域を選択すると最初の接続先を表示します。',empty:'接続記録なし',center:'このMac'},en:{title:'Connection landscape',help:'A schematic based on address ranges. It does not show countries, actual routes or traffic volume. Select an area to show its first destination.',empty:'No connection records',center:'This Mac'},'zh-CN':{title:'连接范围',help:'按地址范围绘制的示意图，不表示国家、实际路径或流量。选择区域以显示首个连接目标。',empty:'无连接记录',center:'此Mac'}}[locale];
  const scopes=['public','local','loopback','other'] as const;
  const categories=scopes.map(kind=>{const targets=rows.flatMap((row,index)=>row.destinations.filter(d=>d.klass===kind).map(destination=>({destination,index})));return {kind,targets,count:targets.reduce((sum,d)=>sum+d.destination.connectionIds.length,0)};});
  return <section className="tg-landscape" aria-label={words.title}><header><span><Globe2 size={17} aria-hidden="true"/><strong>{words.title}</strong></span><span className="info-tip" tabIndex={0} title={words.help} aria-label={words.help}><Info size={16}/></span></header><div className="tg-landscape-body"><div className="tg-landscape-origin"><Laptop size={22} aria-hidden="true"/><span>{words.center}</span></div><div className="tg-landscape-branches">{categories.map(({kind,targets,count})=>{const Icon=classIcons[kind];return <button key={kind} type="button" className="tg-landscape-node" data-scope={kind} disabled={!targets.length} onClick={()=>targets[0]&&selectDestination(targets[0].destination,targets[0].index)} aria-label={`${t(classLabels[kind])}: ${count}`}><Icon size={19} aria-hidden="true"/><span>{t(classLabels[kind])}</span><b>{count.toLocaleString(locale)}</b><i aria-hidden="true"/></button>})}</div></div>{!rows.length&&<small>{words.empty}</small>}</section>;
}
function PageButtons({current,pages,change,locale}:{current:number;pages:number;change:(value:number)=>void;locale:Snapshot['document']['preferences']['locale']}) {
  const w=trafficGraphWords[locale];
  return <div className="tg-page-buttons"><button className="icon-button" disabled={current===0} onClick={()=>change(current-1)} aria-label={w.previous}><ChevronLeft size={19}/></button><span aria-label={w.page}>{current+1} / {pages}</span><button className="icon-button" disabled={current+1>=pages} onClick={()=>change(current+1)} aria-label={w.next}><ChevronRight size={19}/></button></div>;
}
function AppBranch({row,snapshot,t,selectApp,selectDestination}:{row:AppRow}&Omit<Props,'rows'>) {
  const [page,setPage]=useState(0);
  const root=useRef<HTMLDivElement>(null), id=useId().replace(/:/g,'');
  const [paths,setPaths]=useState<string[]>([]);
  const w=trafficGraphWords[snapshot.document.preferences.locale];
  const window=pageWindow(row.destinations,page,destinationPageSize);
  const scopeLabel=(['public','local','loopback','other'] as const).map(klass=>`${t(classLabels[klass])}: ${row.destinations.filter(d=>d.klass===klass).reduce((sum,d)=>sum+d.connectionIds.length,0)}`).join(' · ');
  const signature=JSON.stringify(window.rows.map(d=>d.key));
  const selectedIndex=row.destinations.findIndex(d=>d.connectionIds.includes(snapshot.selectedId??''));
  useEffect(()=>{if(selectedIndex>=0)setPage(Math.floor(selectedIndex/destinationPageSize));},[snapshot.selectedId]);
  useLayoutEffect(()=>{
    const el=root.current;if(!el)return;
    function measure(){
      const el=root.current;if(!el)return;
      const box=el.getBoundingClientRect(),source=el.querySelector('.tg-source')!.getBoundingClientRect();
      const vertical=getComputedStyle(el).gridTemplateColumns.split(' ').length===1;
      setPaths([...el.querySelectorAll('.tg-endpoint')].map((node,index)=>{
        const dest=node.getBoundingClientRect(),y=dest.top+dest.height/2-box.top;
        if(vertical){const sx=source.left+source.width/2-box.left,sy=source.bottom-box.top;return `M ${sx} ${sy} V ${sy+12} H ${4+index*5} V ${y} H ${dest.left-box.left}`;}
        const sx=source.right-box.left,sy=source.top+source.height/2-box.top,dx=dest.left-box.left,middle=(sx+dx)/2;
        return `M ${sx} ${sy} C ${middle} ${sy}, ${middle} ${y}, ${dx} ${y}`;
      }));
    }
    const observer=new ResizeObserver(measure);observer.observe(el);el.querySelectorAll('.tg-source,.tg-endpoint').forEach(node=>observer.observe(node));measure();return()=>observer.disconnect();
  },[signature]);
  return <section className="tg-app-group" aria-label={row.app}>
    <div className="tg-branch" ref={root}>
      <svg className="tg-edges" aria-hidden="true"><defs>{(['allow','block','ask','none'] as const).map(intent=><marker key={intent} id={`${id}-${intent}`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 1 1 L 7 4 L 1 7" fill="none" className={`tg-marker tg-intent-${intent}`} strokeWidth="1.5"/></marker>)}</defs>{window.rows.map((d,index)=>{
        const receipt=destinationReceipt(d,snapshot),selected=d.connectionIds.includes(snapshot.selectedId??'');
        return <g key={d.key}>{selected&&<path d={paths[index]??''} className="tg-focus-halo"/>}<path d={paths[index]??''} className={`tg-edge tg-intent-${d.intent} ${receipt==='confirmed'?'confirmed':'pending'} ${selected?'selected':''}`} markerStart={d.direction==='inbound'?`url(#${id}-${d.intent})`:undefined} markerEnd={d.direction==='outbound'?`url(#${id}-${d.intent})`:undefined}/></g>;
      })}</svg>
      <button className="tg-source" aria-pressed={selectedIndex>=0} onClick={()=>selectApp(row)} title={row.appKey||t('noIdentity')}>
        <AppIcon app={row.app} appKey={row.appKey}/><strong className="tg-app-name" title={row.app}>{row.app}</strong>
        <span className="tg-app-count"><b>{row.connectionCount}</b><span>{w.sockets}</span></span>
        <span className="tg-scope-strip" role="img" aria-label={scopeLabel} title={scopeLabel}>{(['public','local','loopback','other'] as const).map(klass=>{const count=row.destinations.filter(d=>d.klass===klass).reduce((sum,d)=>sum+d.connectionIds.length,0);return count>0?<i key={klass} data-scope={klass} style={{flex:count}}/>:null;})}</span>
        <span className="tg-app-meta">{row.processCount} {w.processes} · {row.destinations.length} {w.destinations}</span>
        <span className="tg-source-port" aria-hidden="true"/>
      </button>
      <div className="tg-destinations">{window.rows.map(d=>{
        const Icon=classIcons[d.klass],receipt=destinationReceipt(d,snapshot),selected=d.connectionIds.includes(snapshot.selectedId??''),name=destinationName(snapshot,d.destination),flowNames=flowNamesFor(snapshot,d.connectionIds).slice(0,8),locale=snapshot.document.preferences.locale,nw=destinationNameWords[locale],fw=flowNameWords[locale];
        return <div key={d.key} className="tg-route-row"><button className={`tg-route-summary ${selected?'selected':''}`} onClick={()=>selectDestination(d)} aria-pressed={selected} aria-label={`${t(d.direction==='inbound'?'inbound':d.direction==='outbound'?'outbound':'unknownDirection')} · ${d.protocol} · ${d.port} · ${d.connectionIds.length} ${w.connections}`} title={`${t(d.direction==='inbound'?'inbound':d.direction==='outbound'?'outbound':'unknownDirection')} · ${d.connectionIds.length} ${w.connections}`}>
          <span><DirectionIcon direction={d.direction}/><strong>{d.protocol}</strong><span>:{d.port}</span></span><span className="tg-route-count"><b>{d.connectionIds.length}</b><small>{w.connections}</small></span>
          </button><button className={`tg-endpoint ${selected?'selected':''}`} data-intent={d.intent} data-receipt={receipt} aria-pressed={selected} onClick={()=>selectDestination(d)}>
          <span className="tg-target-port" aria-hidden="true"/>
          <span className="tg-target-top"><span className="tg-scope-icon" data-scope={d.klass} title={t(classLabels[d.klass])}><Icon size={22} aria-hidden="true"/></span><span className="tg-address-stack"><span className="tg-address" title={d.destination}>{d.destination}</span>{name?.value&&<span className="tg-domain" title={`${nw.resolver} · ${nw.unverified}${name.stale?` · ${nw.stale}`:''} · ${nw.time}: ${new Date(name.observedAt*1000).toLocaleString(locale)}`}><span aria-hidden="true">↳ </span>{name.value}<span className="sr-only"> · {nw.resolver} · {nw.unverified}{name.stale?` · ${nw.stale}`:''}</span></span>}{flowNames.map((flow,i)=><span className="tg-domain" key={`${flow.value}-${i}`} title={`${fw.source} · ${fw.unverified} · ${fw.time}: ${new Date(flow.observedAt*1000).toLocaleString(locale)}`}><span aria-hidden="true">↳ </span>{flow.value}<span className="sr-only"> · {fw.source} · {fw.unverified}</span></span>)}</span><strong className="tg-port">{d.port}</strong></span>
          <span className="tg-protocol"><span>{d.protocol}</span><DirectionIcon direction={d.direction}/><span>{t(d.direction==='inbound'?'inbound':d.direction==='outbound'?'outbound':'unknownDirection')}</span><span className="tg-scope-label" data-scope={d.klass} title={w.scope}>{t(classLabels[d.klass])}</span><span className="tg-socket-count">×{d.connectionIds.length}</span></span>
          <DestinationNameEvidence row={name&&!name.value?name:undefined} locale={locale}/>
          <span className="tg-status-row"><span className="tg-intent-status"><small>{w.intent}</small><strong className={`tg-intent-${d.intent}`}>{d.intent==='none'?<Minus size={20}/>:<ActionIcon action={d.intent} size={20}/>} {d.intent==='none'?w.noRule:t(d.intent)}</strong></span><span className="tg-receipt-status"><small>{w.receipt}</small><span>{receipt==='confirmed'?<Check size={15}/>:receipt==='protected'?<ShieldCheck size={15}/>:<CircleHelp size={15}/>} {w[receipt]}</span></span></span>
        </button></div>;
      })}
        {window.pages>1&&<nav className="tg-destination-paging" aria-label={`${row.app} · ${w.more}`}><span>{window.start+1}–{window.start+window.rows.length} / {row.destinations.length}</span>{selectedIndex>=0&&Math.floor(selectedIndex/destinationPageSize)!==window.current&&<button className="text-button" onClick={()=>setPage(Math.floor(selectedIndex/destinationPageSize))}>{w.follow}</button>}<PageButtons current={window.current} pages={window.pages} change={setPage} locale={snapshot.document.preferences.locale}/></nav>}
      </div>
    </div>
  </section>;
}

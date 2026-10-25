import {useId,useLayoutEffect,useRef,useState} from 'react';
import {AppWindow,ArrowDownUp,Globe2,SlidersHorizontal,Shield,FileCheck2,Info,Check,Minus,TriangleAlert} from 'lucide-react';
import {controlGraphState,controlGraphWords,type ControlNode} from '../core/control-graph';
import {extensionLabel} from '../core/enforcement';
import {translator} from '../core/i18n';
import type {Snapshot} from '../core/types';
import './ControlGraph.css';
const nodes=[['apps',AppWindow],['traffic',ArrowDownUp],['destinations',Globe2],['rules',SlidersHorizontal],['provider',Shield],['receipt',FileCheck2]] as const;
const edges=[['apps','traffic','data'],['traffic','destinations','data'],['rules','provider','control'],['provider','receipt','control'],['provider','traffic','bridge']] as const;
type Edge={from:ControlNode;to:ControlNode;kind:string;d:string};
export function ControlGraph({snapshot}:{snapshot:Snapshot}){
 const [selected,setSelected]=useState<ControlNode>('traffic'),[paths,setPaths]=useState<Edge[]>([]);
 const canvas=useRef<HTMLDivElement>(null),id=useId().replace(/:/g,'');
 const w=controlGraphWords[snapshot.document.preferences.locale],t=translator(snapshot.document.preferences.locale),state=controlGraphState(snapshot);
 useLayoutEffect(()=>{
  const root=canvas.current;if(!root)return;
  const measure=()=>{const rect=root.getBoundingClientRect(),vertical=getComputedStyle(root).gridTemplateColumns.split(' ').length===1;
   const box=(node:ControlNode)=>root.querySelector<HTMLButtonElement>(`[data-node="${node}"]`)!.getBoundingClientRect();
   setPaths(edges.map(([from,to,kind])=>{const a=box(from),b=box(to);let d:string;
    if(kind==='bridge'){
     if(vertical){const x=12,ay=a.top+a.height/2-rect.top,by=b.top+b.height/2-rect.top;d=`M ${a.left-rect.left} ${ay} H ${x} V ${by} H ${b.left-rect.left}`;}
     else {const x=a.left+a.width/2-rect.left;d=`M ${x} ${a.top-rect.top} V ${b.bottom-rect.top}`;}
    }else if(vertical){const x=a.left+a.width/2-rect.left;d=`M ${x} ${a.bottom-rect.top} V ${b.top-rect.top}`;}
    else{const y=a.top+a.height/2-rect.top;d=`M ${a.right-rect.left} ${y} H ${b.left-rect.left}`;}
    return {from,to,kind,d};}));};
  const observer=new ResizeObserver(measure);observer.observe(root);root.querySelectorAll('[data-node]').forEach(n=>observer.observe(n));measure();return()=>observer.disconnect();
 },[]);
 const observation=state.demo?w.sample:state.observed?w.observed:w.unknown;
 const receipt=state.demo?w.sample:state.acknowledged?w.confirmed:state.uncertain?w.uncertain:w.pending;
 const values:Record<ControlNode,string>={apps:String(state.applications),traffic:String(state.connections),destinations:String(state.destinations),rules:String(state.rules.length),provider:state.demo?w.sample:t(extensionLabel(snapshot.extension.stage)),receipt};
 const badges:Record<ControlNode,string>={apps:observation,traffic:observation,destinations:observation,rules:state.demo?w.sample:w.saved,provider:state.demo?w.sample:state.ready?w.ready:state.uncertain?w.uncertain:w.unknown,receipt:state.acknowledged?`revision ${snapshot.extension.providerRevision}`:w.unknown};
 const connected=(node:ControlNode)=>edges.some(([a,b])=>a===selected&&b===node||b===selected&&a===node);
 const error=state.uncertain&&['provider','receipt'].includes(selected);
 const exampleNames=selected==='apps'?[...new Set(snapshot.applications.map(a=>a.app))]:selected==='destinations'?[...new Set(snapshot.connections.map(c=>c.destination))]:[];
 return <div className="control-graph">
  <div className="cg-toolbar"><div className="cg-legend"><span><i className="cg-line"/>{w.data}</span><span><i className="cg-line control"/>{w.control}</span></div><span className="cg-mode">{state.demo?w.sample:w.current}</span><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span></div>
  <div className="cg-canvas" ref={canvas} role="group" aria-label={w.title}>
   <svg className="cg-edges" aria-hidden="true"><defs><marker id={`${id}-arrow`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 1 1 L 7 4 L 1 7" fill="none" stroke="context-stroke" strokeWidth="1.5"/></marker></defs>{paths.map(edge=><path key={edge.from+edge.to} d={edge.d} className={`cg-edge ${edge.kind} ${[edge.from,edge.to].includes(selected)?'selected':''} ${edge.kind==='bridge'&&state.acknowledged?'acknowledged':''}`} markerEnd={`url(#${id}-arrow)`} markerStart={edge.kind==='data'?`url(#${id}-arrow)`:undefined}/>)}</svg>
   <div className="cg-lane-label data">{w.scope}<span>{w.data}</span></div>
   {nodes.map(([key,Icon])=><button key={key} type="button" data-node={key} className={`cg-node ${selected===key?'selected':''} ${connected(key)?'related':''} ${['provider','receipt'].includes(key)&&state.uncertain?'uncertain':''} ${key==='receipt'&&state.acknowledged?'acknowledged':''}`} aria-pressed={selected===key} aria-controls={`${id}-detail`} onClick={()=>setSelected(key)}>
    <span className="cg-node-heading"><Icon size={18} aria-hidden="true"/><strong>{w.nodes[key]}</strong></span>
    <span className={`cg-value ${['provider','receipt'].includes(key)?'status':''}`}>{values[key]}</span>
    <span className="cg-badge">{key==='receipt'&&state.acknowledged?<Check size={13}/>:['provider','receipt'].includes(key)&&state.uncertain?<TriangleAlert size={13}/>:<Minus size={13}/>}<span>{badges[key]}</span></span>
   </button>)}
   <div className="cg-lane-label control">FlowGuard<span>{w.control}</span></div>
  </div>
  <div id={`${id}-detail`} className={`cg-detail ${error?'uncertain':''}`} aria-live="polite"><div className="cg-detail-heading"><strong>{w.nodes[selected]}</strong><span>{w.select}</span></div><p>{w.details[selected]}</p>
   {selected==='rules'&&<div className="cg-facts">{(['allow','block','ask'] as const).map(action=><span key={action}>{w[action]} <b>{state.actions[action]}</b></span>)}</div>}
   {exampleNames.length>0&&<p className="cg-evidence">{selected==='apps'?w.names:w.targets}: {exampleNames.slice(0,6).join(' · ')}{exampleNames.length>6?' …':''}</p>}
   {selected==='traffic'&&<p className="cg-evidence">{w.time}: {state.observed&&snapshot.lastObserved!==null?new Date(snapshot.lastObserved*1000).toLocaleString(snapshot.document.preferences.locale):state.demo?w.sample:w.unknown}</p>}
   {selected==='provider'&&<p className="cg-evidence">{state.demo?w.sample:t(extensionLabel(snapshot.extension.stage))}</p>}
   {selected==='receipt'&&<p className="cg-evidence">{receipt}{state.acknowledged?` · ${w.revision}: ${snapshot.extension.providerRevision}`:''}</p>}
  </div>
 </div>;
}

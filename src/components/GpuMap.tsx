import {useEffect,useRef,useState} from 'react';
import {Info,Network,X,Plus,Minus,ArrowUp,ArrowDown,RotateCcw,ExternalLink} from 'lucide-react';
import {invoke} from '@tauri-apps/api/core';
import {native} from '../core/store';
import {clipBounds,gpuWords,type GpuStatus,type GpuFilter,type Rect} from '../core/gpu-panel';
import type {Snapshot} from '../core/types';
function rect(r:DOMRect):Rect{return {x:r.x,y:r.y,width:r.width,height:r.height};}
export function GpuMap({snapshot,filter,standalone=false}:{snapshot:Snapshot;filter:GpuFilter;standalone?:boolean}){
 const w=gpuWords[snapshot.document.preferences.locale];
 const [available,setAvailable]=useState(false),[open,setOpen]=useState(standalone),[failed,setFailed]=useState(false),[status,setStatus]=useState<{token:string;value:GpuStatus}|null>(null);
 const token=JSON.stringify([filter,snapshot.revision]);
 const desired=useRef({filter,token});desired.current={filter,token};
 const current=status?.token===token?status.value:null;
 const interaction=useRef<{token:string;value:{type:string;x?:number;y?:number}}|null>(null);
 function interact(value:{type:string;x?:number;y?:number}){interaction.current={token,value};request.current?.();}
 useEffect(()=>{if(standalone&&snapshot.gpuFloatOperation)setOpen(true);if(!standalone&&snapshot.gpuDockOperation)setOpen(true);},[standalone,snapshot.gpuFloatOperation,snapshot.gpuDockOperation]);
 useEffect(()=>{if(snapshot.gpuPanelOwner&&snapshot.gpuPanelOwner!==(standalone?'gpu-map':'main')){setOpen(false);setStatus(null);}},[standalone,snapshot.gpuPanelOwner]);
 const slot=useRef<HTMLDivElement>(null),request=useRef<(()=>void)|null>(null);
 useEffect(()=>{let alive=true;if(native)void invoke<boolean>('gpu_panel_available').then(v=>{if(alive)setAvailable(v);}).catch(()=>{});return()=>{alive=false;};},[]);
 useEffect(()=>{if(!open||!available)return;
  let cancelled=false,lease:number|null=null,sequence=0,frame=0,inFlight=false,dirty=false;
  const send=async()=>{frame=0;if(cancelled||lease===null)return;if(inFlight){dirty=true;return;}inFlight=true;
   const element=slot.current;let bounds=null;
   if(element&&document.visibilityState==='visible'){
    const clips:Rect[]=[];for(let parent=element.parentElement;parent;parent=parent.parentElement){const s=getComputedStyle(parent);if(/auto|scroll|hidden|clip/.test(s.overflowX+' '+s.overflowY))clips.push(rect(parent.getBoundingClientRect()));}
    const bar=document.querySelector('.protection-status')?.getBoundingClientRect();if(bar&&bar.bottom>0)clips.push({x:0,y:bar.bottom,width:innerWidth,height:Math.max(0,innerHeight-bar.bottom)});
    bounds=clipBounds(rect(element.getBoundingClientRect()),innerWidth,innerHeight,clips);
   }
   const target=desired.current;const action=interaction.current;interaction.current=null;
   try{const next=await invoke<GpuStatus>('sync_gpu_panel',{lease,sequence:++sequence,bounds,filter:target.filter,interaction:action?.token===target.token?action.value:null});if(!cancelled){if(target.token===desired.current.token)setStatus({token:target.token,value:next});else dirty=true;}}
   catch(error){if(!cancelled){if(target.token!==desired.current.token)dirty=true;else if(String(error).includes('stale_gpu_source'))setStatus(null);else if(String(error).includes('stale_gpu_panel')||String(error).includes('stale_gpu_owner')){setOpen(false);setStatus(null);}else{setFailed(true);setOpen(false);}}}
   finally{inFlight=false;if(dirty&&!cancelled){dirty=false;schedule();}}
  };
  function schedule(){if(!cancelled&&!frame)frame=requestAnimationFrame(()=>void send());}
  request.current=schedule;
  void invoke<number>('open_gpu_panel').then(id=>{lease=id;if(cancelled){void invoke('close_gpu_panel',{lease:id}).catch(()=>{});}else schedule();}).catch(()=>{if(!cancelled){setFailed(true);setOpen(false);}});
  const observer=new ResizeObserver(schedule);if(slot.current)observer.observe(slot.current);
  window.addEventListener('scroll',schedule,true);window.addEventListener('resize',schedule);window.addEventListener('focus',schedule);document.addEventListener('visibilitychange',schedule);
  const timer=window.setInterval(schedule,1000);
  return()=>{cancelled=true;request.current=null;observer.disconnect();cancelAnimationFrame(frame);clearInterval(timer);window.removeEventListener('scroll',schedule,true);window.removeEventListener('resize',schedule);window.removeEventListener('focus',schedule);document.removeEventListener('visibilitychange',schedule);if(lease!==null)void invoke('close_gpu_panel',{lease}).catch(()=>{});};
 },[open,available]);
 useEffect(()=>{request.current?.();},[token]);
 return <section className="gpu-map surface"><div className="gpu-map-head"><Network size={18}/><strong>{w.title}</strong><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span><button className="text-button" disabled={!available} title={available?w.help:w.unavailable} onClick={()=>{setFailed(false);setStatus(null);setOpen(v=>!v);}}>{open?<><X size={16}/>{w.close}</>:w.open}</button></div>
 {failed&&<p role="alert">{w.failed}</p>}
 {!standalone&&available&&<button className="text-button" title={w.float} onClick={()=>void invoke('float_gpu_panel',{filter}).catch(()=>setFailed(true))}><ExternalLink size={16}/>{w.float}</button>}
 {open&&<><div className="gpu-map-tools">{([['zoom_in',Plus,w.zoomIn],['zoom_out',Minus,w.zoomOut],['pan_up',ArrowUp,w.panUp],['pan_down',ArrowDown,w.panDown],['reset',RotateCcw,w.reset]] as const).map(([type,Icon,label])=><button className="icon-button" key={type} aria-label={label} title={label} onClick={()=>interact({type})}><Icon size={17}/></button>)}</div><div className="gpu-map-axis"><span>{w.apps}</span><span>{w.destinations}</span></div><div className="gpu-map-slot" ref={slot} role="img" tabIndex={0} aria-label={w.interact} title={w.interact} onClick={e=>{const r=e.currentTarget.getBoundingClientRect();interact({type:'pick',x:(e.clientX-r.left)/r.width,y:(e.clientY-r.top)/r.height});}} onKeyDown={e=>{const action=({'+':'zoom_in','-':'zoom_out',ArrowUp:'pan_up',ArrowDown:'pan_down',Home:'reset'} as Record<string,string>)[e.key];if(action){e.preventDefault();interact({type:action});}}}/><p role="status" className="gpu-map-status">{current?.presented?<>{w.routes}: {current.routes.toLocaleString()} / {w.matched}: {current.matched.toLocaleString()} / {w.total}: {current.total.toLocaleString()}{current.matched===0&&<> · {w.empty}</>}{current.truncated&&<> · {w.limit}</>}</>:w.waiting}</p></>}
 </section>;
}

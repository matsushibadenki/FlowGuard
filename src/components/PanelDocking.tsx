import {useEffect,useState} from 'react';
import {GripVertical,PanelLeft,PanelRight,ExternalLink} from 'lucide-react';
import {dockPanel,floatPanel,native} from '../core/store';
import {acceptsPanel,panelDragType,panelDragValue,dockingWords} from '../core/panel-docking';
import type {Locale} from '../core/types';
export function PanelDragHandle({locale,onError}:{locale:Locale;onError:(e:unknown)=>void}){
 const w=dockingWords[locale];
 return <button className="icon-button panel-drag-handle" draggable aria-label={w.drag} title={w.drag} onDragStart={e=>{e.dataTransfer.setData(panelDragType,panelDragValue);e.dataTransfer.effectAllowed='move';window.dispatchEvent(new Event('flowguard-panel-drag'));}} onKeyDown={e=>{if(!['ArrowLeft','ArrowRight','Enter'].includes(e.key))return;e.preventDefault();void (e.key==='Enter'?floatPanel():dockPanel(e.key==='ArrowLeft'?'left':'right')).catch(onError);}}><GripVertical size={18}/></button>;
}
export function PanelDockTargets({locale,onError}:{locale:Locale;onError:(e:unknown)=>void}){
 const [active,setActive]=useState(false);const w=dockingWords[locale];
 useEffect(()=>{const start=()=>setActive(true),over=(e:DragEvent)=>{if(acceptsPanel([...e.dataTransfer?.types??[]]))setActive(true);},end=()=>setActive(false),leave=(e:DragEvent)=>{if(!e.relatedTarget)setActive(false);};window.addEventListener('flowguard-panel-drag',start);window.addEventListener('dragover',over);window.addEventListener('dragend',end);window.addEventListener('drop',end);window.addEventListener('dragleave',leave);return()=>{window.removeEventListener('flowguard-panel-drag',start);window.removeEventListener('dragover',over);window.removeEventListener('dragend',end);window.removeEventListener('drop',end);window.removeEventListener('dragleave',leave);};},[]);
 if(!active)return null;
 return <div className="panel-dock-targets">{(['left','right','float'] as const).map((target,i)=>{const Icon=[PanelLeft,PanelRight,ExternalLink][i];return <div key={target} className={`panel-drop-zone ${target}`} role="region" aria-label={w[target]} onDragOver={e=>{if(acceptsPanel([...e.dataTransfer.types])&&(target!=='float'||native)){e.preventDefault();e.dataTransfer.dropEffect='move';}}} onDrop={e=>{e.preventDefault();setActive(false);if(!acceptsPanel([...e.dataTransfer.types],e.dataTransfer.getData(panelDragType)))return;void (target==='float'?floatPanel():dockPanel(target)).catch(onError);}}><Icon size={24}/><span>{w[target]}</span></div>;})}</div>;
}

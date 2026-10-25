import {useEffect,useState,useRef} from 'react';
import {ArrowUp,ArrowDown,Check,Copy,Info,Columns3,Globe2,Network,Monitor,Clock,ScanSearch} from 'lucide-react';
import {editLanInventory,copyLanSelection} from '../core/store';
import {lanColumns,lanColumnLabels,defaultLanWidth,type LanEdit} from '../core/lan-workbench';
import {metadataStatus,columnWord,scopeOf,sheetWords,virtualWindow} from '../core/lan-sheet';
import type {Snapshot,DeviceChange} from '../core/types';
import type {T} from './ui';
import {changeKeys} from './LanDetails';
import './LanSheet.css';
function Annotation({snapshot,id,t,disabled,onError,label=false}:{snapshot:Snapshot;id:string;t:T;disabled:boolean;onError:()=>void;label?:boolean}){
 const value=(label?snapshot.inventory.labels:snapshot.inventory.comments)?.[id]??'';
 const [draft,setDraft]=useState(value),[base,setBase]=useState(value),[dirty,setDirty]=useState(false),[busy,setBusy]=useState(false);
 useEffect(()=>{if(!dirty){setDraft(value);setBase(value);}},[value,dirty]);
 const title=label?sheetWords[snapshot.document.preferences.locale].label:t('lanComment');
 async function save(){setBusy(true);try{await editLanInventory(snapshot.inventory.generation,{type:label?'label':'comment',id,value:draft,expected:base});setDirty(false);}catch{onError();}finally{setBusy(false);}}
 return <div><div className="lan-comment-editor"><textarea aria-label={`${title}: ${id}`} value={draft} maxLength={label?80:500} rows={2} disabled={disabled||busy} onChange={e=>{setDraft(e.target.value);setDirty(true);}}/>
 <button className="icon-button" aria-label={`${t('lanSaveComment')}: ${title} ${id}`} title={t('lanSaveComment')} disabled={!dirty||disabled||busy} onClick={()=>void save()}><Check size={16}/></button></div>{dirty&&base!==value&&<button className="text-button" disabled={busy} onClick={()=>{setDraft(value);setBase(value);setDirty(false);}}>{t('lanCommentReload')}</button>}</div>;
}
export function LanTable({snapshot,rows,t,native,time,onSelect,sortColumn='address',descending=false,onSort,readonly=false}:{snapshot:Snapshot;rows:DeviceChange[];t:T;native:boolean;time:(n:number)=>string;onSelect:(id:string)=>Promise<unknown>;sortColumn?:string;descending?:boolean;onSort?:(column:string)=>void;readonly?:boolean}){
 const inventory=snapshot.inventory,checked=inventory.checkedIds??[],columns=inventory.columns??[...lanColumns],w=sheetWords[snapshot.document.preferences.locale];
 const [pending,setPending]=useState(false),[error,setError]=useState(false),[copied,setCopied]=useState(false),[scroll,setScroll]=useState(0);
 const container=useRef<HTMLDivElement>(null),[availableWidth,setAvailableWidth]=useState(10000);
 useEffect(()=>{const el=container.current;if(!el)return;const observer=new ResizeObserver(()=>setAvailableWidth(el.clientWidth));observer.observe(el);setAvailableWidth(el.clientWidth);return()=>observer.disconnect();},[rows.length>0]);
 const width=(column:string)=>{const saved=inventory.widths?.[column]??defaultLanWidth(column);return column==='address'?Math.min(saved,Math.max(100,availableWidth-86)):saved;};
 useEffect(()=>{setScroll(0);if(container.current)container.current.scrollTop=0;},[rows.length,sortColumn,descending,inventory.generation]);
 const busy=readonly||inventory.status==='running'||snapshot.updates.stage==='installing'||pending;
 const title=(column:string)=>columnWord(column,snapshot.document.preferences.locale)??t(lanColumnLabels[column]);
 async function edit(command:LanEdit){setPending(true);setError(false);setCopied(false);try{await editLanInventory(inventory.generation,command);}catch{setError(true);}finally{setPending(false);}}
 async function copy(){setPending(true);setError(false);setCopied(false);try{await copyLanSelection(inventory.generation);setCopied(true);}catch{setError(true);}finally{setPending(false);}}
 const toggle=(id:string)=>void edit({type:'selection',ids:checked.includes(id)?checked.filter(x=>x!==id):[...checked,id],expected:checked});
 function move(index:number,offset:number){const values=[...columns];[values[index],values[index+offset]]=[values[index+offset],values[index]];void edit({type:'columns',values,expected:columns});}
 const cell=(row:DeviceChange,column:string)=>{
  const d=row.device;
  switch(column){
   case 'change':return <span className={`change-label change-${row.kind}`}>{t(changeKeys[row.kind as keyof typeof changeKeys]??'lanUncompared')}</span>;
   case 'inspect':return <button className="icon-button" aria-label={`${t('lanDetails')}: ${d.address}`} title={t('lanInspect')} onClick={()=>void onSelect(d.id).catch(()=>setError(true))}><ScanSearch size={19}/></button>;
   case 'address':return <span className="lan-address">{d.address}</span>;
   case 'family':return d.family;
   case 'interface':return d.interface;
   case 'scope':{const scope=scopeOf(d.address),Icon=scope==='public'?Globe2:scope==='local'?Network:Monitor;return <span className={`lan-scope lan-scope-${scope}`}><Icon size={16}/>{w[scope]}</span>;}
   case 'latency':{const ms=snapshot.discovery.responseMs?.[d.id];return ms===undefined?'—':<span className="lan-rtt"><b>{ms.toLocaleString(snapshot.document.preferences.locale,{maximumFractionDigits:2})}</b> ms<i style={{width:`${Math.min(100,Math.max(3,ms/10))}%`}}/></span>;}
   case 'firstSeen':return inventory.firstSeen?.[d.id]?<span><Clock size={13}/> {time(inventory.firstSeen[d.id])}</span>:'—';
   case 'services':{const m=snapshot.discovery.metadata?.[d.id];return m?<span title={`${w.metadataHelp} · ${time(m.observedAt)}`}>{m.services.length?m.services.map(s=><small key={`${s.instance}:${s.port}`} className="name-time">{s.service.split('.')[0]} · {s.port}</small>):m.serviceStatus==='not_requested'?'—':`${w.empty} · ${metadataStatus(m.serviceStatus,snapshot.document.preferences.locale)}`}</span>:'—';}
   case 'smb':{const m=snapshot.discovery.metadata?.[d.id];return m?<span title={`${w.metadataHelp} · ${time(m.observedAt)}`}>{[...m.smbNames,...m.smbDomains].join(' · ')||(m.smbStatus==='not_requested'?'—':`${w.empty} · ${metadataStatus(m.smbStatus,snapshot.document.preferences.locale)}`)}</span>:'—';}
   case 'vendor':{const v=inventory.vendors?.[d.id];return v?<span title={`${v.source} · ${v.retrievedAt} · ${w.vendorHelp}`}>{v.organization}<small className="name-time">{v.source}</small></span>:'—';}
   case 'mac':return d.mac??t('lanNotCollected');
   case 'name':return <span title={(d.name.values??[]).join('\n')}>{d.name.value??t(d.name.status==='pending'?(snapshot.discovery.stage==='running'?'lanNamePending':'lanNameInterrupted'):d.name.status==='unobserved'?'lanNameUnobserved':d.name.status==='failed'?'lanNameFailed':d.name.status==='skipped'?'lanNameSkipped':d.name.status==='limited'?'lanNamesLimited':'lanNotCollected')}{(d.name.values?.length??0)>1&&<small> +{d.name.values!.length-1}</small>}</span>;
   case 'nameSource':return <>{d.name.source==='system'?t('lanSystemSource'):d.name.source==='mdns'?'mDNS':t('lanNotCollected')}{d.name.observedAt&&<small className="name-time">{time(d.name.observedAt)}</small>}</>;
   case 'response':return <span className={`lan-response lan-response-${d.response}`}><i/>{t(d.response==='responded'?'lanResponded':d.response==='no_response'?'lanNoResponse':d.response==='local'?'lanLocal':'lanUnverified')}</span>;
   case 'source':return d.source==='local'?t('lanLocal'):d.source.toUpperCase();
   case 'observed':return time(d.observedAt);
   case 'label':case 'comment':return readonly?<span>{(column==='label'?inventory.labels:inventory.comments)?.[d.id]??'—'}</span>:<Annotation key={`${inventory.generation}:${d.id}:${column}`} label={column==='label'} snapshot={snapshot} id={d.id} t={t} disabled={busy} onError={()=>setError(true)}/>;
   default:return null;
  }
 };
 const virtual=rows.length>80,window=virtual?virtualWindow(rows.length,scroll):{start:0,end:rows.length,top:0,bottom:0};
 return <><div className="lan-workbench-toolbar">
  <span>{t('lanSelectedCount').replace('{n}',String(checked.length))} <span tabIndex={0} title={t('lanCopyHelp')} aria-label={t('lanCopyHelp')}><Info size={15}/></span></span>
  <button className="secondary-button" disabled={busy||!rows.length} onClick={()=>void edit({type:'selection',ids:[...new Set([...checked,...rows.map(r=>r.device.id)])],expected:checked})}>{t('lanSelectVisible')}</button>
  <button className="text-button" disabled={busy||!checked.length} onClick={()=>void edit({type:'selection',ids:[],expected:checked})}>{t('lanClearSelection')}</button>
  <button className="icon-button" disabled={!native||busy||!checked.length} aria-label={t('lanCopySelected')} title={t('lanCopyHelp')} onClick={()=>void copy()}><Copy size={18}/></button>
  <details className="lan-columns"><summary><Columns3 size={16}/>{t('lanColumns')}</summary><div>{[...columns,...lanColumns.filter(c=>!columns.includes(c))].map(column=>{const index=columns.indexOf(column);return <div key={column}><label><input type="checkbox" checked={index>=0} disabled={busy||column==='address'||column==='inspect'} onChange={()=>void edit({type:'columns',values:index>=0?columns.filter(c=>c!==column):[...columns,column],expected:columns})}/>{title(column)}</label>
   <select aria-label={`${w.width}: ${title(column)}`} disabled={busy} value={inventory.widths?.[column]??defaultLanWidth(column)} onChange={e=>void edit({type:'width',column,value:Number(e.target.value),expected:inventory.widths?.[column]??defaultLanWidth(column)})}>{[80,120,160,220,320,480].map(width=><option key={width} value={width}>{width}px</option>)}</select>
   <button className="icon-button" aria-label={`${t('lanColumnUp')}: ${title(column)}`} disabled={busy||index<=0} onClick={()=>move(index,-1)}><ArrowUp size={15}/></button><button className="icon-button" aria-label={`${t('lanColumnDown')}: ${title(column)}`} disabled={busy||index<0||index===columns.length-1} onClick={()=>move(index,1)}><ArrowDown size={15}/></button></div>;})}<button className="text-button" disabled={busy} onClick={()=>void edit({type:'columns',values:[...lanColumns],expected:columns})}>{t('lanColumnsReset')}</button></div></details>
  <span tabIndex={0} className="protection-info" title={`${t('lanCommentHelp')} ${w.labelHelp}`} aria-label={`${t('lanCommentHelp')} ${w.labelHelp}`}><Info size={16}/></span>
 </div>{error&&<p role="alert">{t('lanEditError')}</p>}{copied&&<p role="status">{t('lanCopiedSnapshot')}</p>}
 {rows.length>0&&<div ref={container} className={`table-scroll tool-table lan-data-grid ${virtual?'lan-virtual':''}`} onScroll={e=>setScroll(e.currentTarget.scrollTop)}><table aria-rowcount={rows.length+1} style={{width:40+columns.reduce((sum,c)=>sum+width(c),0)}}><colgroup><col style={{width:40}}/>{columns.map(c=><col key={c} style={{width:width(c)}}/>)}</colgroup><thead><tr><th scope="col"><span className="sr-only">{t('lanSelection')}</span></th>{columns.map(column=><th key={column} data-column={column} scope="col" aria-sort={sortColumn===column?(descending?'descending':'ascending'):'none'}><button title={title(column)} disabled={!onSort||column==='inspect'} onClick={()=>onSort?.(column)}><span>{title(column)}</span>{sortColumn===column&&(descending?<ArrowDown size={13}/>:<ArrowUp size={13}/>)}</button></th>)}</tr></thead><tbody>
 {window.top>0&&<tr aria-hidden="true"><td colSpan={columns.length+1} style={{height:window.top,padding:0}}/></tr>}
 {rows.slice(window.start,window.end).map((row,index)=><tr key={row.device.id} aria-rowindex={window.start+index+2} aria-selected={inventory.selectedId===row.device.id} className={`${inventory.selectedId===row.device.id?'selected-record':''} ${virtual?'lan-virtual-row':''}`}>
 <td><input type="checkbox" aria-label={`${t('lanSelection')}: ${row.device.address}`} checked={checked.includes(row.device.id)} disabled={busy} onChange={()=>toggle(row.device.id)}/></td>{columns.map(column=><td key={column} data-column={column}><div className="lan-cell" title={typeof cell(row,column)==='string'?String(cell(row,column)):undefined}>{cell(row,column)}</div></td>)}</tr>)}
 {window.bottom>0&&<tr aria-hidden="true"><td colSpan={columns.length+1} style={{height:window.bottom,padding:0}}/></tr>}
 </tbody></table></div>}
 </>;
}

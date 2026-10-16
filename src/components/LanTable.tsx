import {useEffect,useState} from 'react';
import {ArrowUp,ArrowDown,Check,Copy,Info,Columns3} from 'lucide-react';
import {editLanInventory,copyLanSelection} from '../core/store';
import {lanColumns,lanColumnLabels,type LanEdit} from '../core/lan-workbench';
import type {Snapshot,DeviceChange} from '../core/types';
import type {T} from './ui';
import {changeKeys} from './LanDetails';
function Comment({snapshot,id,t,disabled,onError}:{snapshot:Snapshot;id:string;t:T;disabled:boolean;onError:()=>void}){
 const comment=snapshot.inventory.comments?.[id]??'';
 const [draft,setDraft]=useState(comment),[base,setBase]=useState(comment),[dirty,setDirty]=useState(false),[busy,setBusy]=useState(false);
 useEffect(()=>{if(!dirty){setDraft(comment);setBase(comment);}},[comment,dirty]);
 async function save(){setBusy(true);try{await editLanInventory(snapshot.inventory.generation,{type:'comment',id,value:draft,expected:base});setDirty(false);}catch{onError();}finally{setBusy(false);}}
 return <div><div className="lan-comment-editor"><textarea aria-label={`${t('lanComment')}: ${id}`} value={draft} maxLength={500} rows={2} disabled={disabled||busy} onChange={e=>{setDraft(e.target.value);setDirty(true);}}/>
 <button className="icon-button" aria-label={`${t('lanSaveComment')}: ${id}`} title={t('lanSaveComment')} disabled={!dirty||disabled||busy} onClick={()=>void save()}><Check size={16}/></button></div>{dirty&&base!==comment&&<button className="text-button" disabled={busy} onClick={()=>{setDraft(comment);setBase(comment);setDirty(false);}}>{t('lanCommentReload')}</button>}</div>;
}
export function LanTable({snapshot,rows,t,native,time,onSelect}:{snapshot:Snapshot;rows:DeviceChange[];t:T;native:boolean;time:(n:number)=>string;onSelect:(id:string)=>Promise<unknown>}){
 const inventory=snapshot.inventory,checked=inventory.checkedIds??[],columns=inventory.columns??[...lanColumns];
 const [pending,setPending]=useState(false),[error,setError]=useState(false),[copied,setCopied]=useState(false);
 const busy=inventory.status==='running'||snapshot.updates.stage==='installing'||pending;
 async function edit(command:LanEdit){setPending(true);setError(false);setCopied(false);try{await editLanInventory(inventory.generation,command);}catch{setError(true);}finally{setPending(false);}}
 async function copy(){setPending(true);setError(false);setCopied(false);try{await copyLanSelection(inventory.generation);setCopied(true);}catch{setError(true);}finally{setPending(false);}}
 const toggle=(id:string)=>void edit({type:'selection',ids:checked.includes(id)?checked.filter(x=>x!==id):[...checked,id],expected:checked});
 function move(index:number,offset:number){const values=[...columns];[values[index],values[index+offset]]=[values[index+offset],values[index]];void edit({type:'columns',values,expected:columns});}
 const cell=(row:DeviceChange,column:string)=>{
  const d=row.device;
  switch(column){
   case 'change':return <span className={`change-label change-${row.kind}`}>{t(changeKeys[row.kind as keyof typeof changeKeys]??'lanUncompared')}</span>;
   case 'inspect':return <button className="secondary-button" aria-label={`${t('lanDetails')}: ${d.address}`} onClick={()=>void onSelect(d.id).catch(()=>setError(true))}>{t('lanInspect')}</button>;
   case 'address':return d.address;
   case 'family':return d.family;
   case 'interface':return d.interface;
   case 'mac':return d.mac??t('lanNotCollected');
   case 'name':return d.name.value??t(d.name.status==='pending'?(snapshot.discovery.stage==='running'?'lanNamePending':'lanNameInterrupted'):d.name.status==='unobserved'?'lanNameUnobserved':d.name.status==='failed'?'lanNameFailed':d.name.status==='skipped'?'lanNameSkipped':d.name.status==='limited'?'lanNamesLimited':'lanNotCollected');
   case 'nameSource':return <>{d.name.source==='system'?t('lanSystemSource'):d.name.source==='mdns'?'mDNS':t('lanNotCollected')}{d.name.observedAt&&<small className="name-time">{time(d.name.observedAt)}</small>}</>;
   case 'response':return t(d.response==='responded'?'lanResponded':d.response==='no_response'?'lanNoResponse':d.response==='local'?'lanLocal':'lanUnverified');
   case 'source':return d.source==='local'?t('lanLocal'):d.source.toUpperCase();
   case 'observed':return time(d.observedAt);
   case 'comment':return <Comment key={`${inventory.generation}:${d.id}`} snapshot={snapshot} id={d.id} t={t} disabled={busy} onError={()=>setError(true)}/>;
   default:return null;
  }
 };
 return <><div className="lan-workbench-toolbar">
  <span>{t('lanSelectedCount').replace('{n}',String(checked.length))} <span tabIndex={0} title={t('lanCopyHelp')} aria-label={t('lanCopyHelp')}><Info size={15}/></span></span>
  <button className="secondary-button" disabled={busy||!rows.length} onClick={()=>void edit({type:'selection',ids:[...new Set([...checked,...rows.map(r=>r.device.id)])],expected:checked})}>{t('lanSelectVisible')}</button>
  <button className="text-button" disabled={busy||!checked.length} onClick={()=>void edit({type:'selection',ids:[],expected:checked})}>{t('lanClearSelection')}</button>
  <button className="secondary-button" disabled={!native||busy||!checked.length} title={!native?t('interfacePreview'):t('lanCopyHelp')} onClick={()=>void copy()}><Copy size={16}/>{t('lanCopySelected')}</button>
  <details className="lan-columns"><summary><Columns3 size={16}/>{t('lanColumns')}</summary><div>{[...columns,...lanColumns.filter(c=>!columns.includes(c))].map(column=>{const index=columns.indexOf(column);return <div key={column}><label><input type="checkbox" checked={index>=0} disabled={busy||column==='address'||column==='inspect'} onChange={()=>void edit({type:'columns',values:index>=0?columns.filter(c=>c!==column):[...columns,column],expected:columns})}/>{t(lanColumnLabels[column])}</label>
   <button className="icon-button" aria-label={`${t('lanColumnUp')}: ${t(lanColumnLabels[column])}`} disabled={busy||index<=0} onClick={()=>move(index,-1)}><ArrowUp size={15}/></button><button className="icon-button" aria-label={`${t('lanColumnDown')}: ${t(lanColumnLabels[column])}`} disabled={busy||index<0||index===columns.length-1} onClick={()=>move(index,1)}><ArrowDown size={15}/></button></div>;})}<button className="text-button" disabled={busy} onClick={()=>void edit({type:'columns',values:[...lanColumns],expected:columns})}>{t('lanColumnsReset')}</button></div></details>
  <span tabIndex={0} className="protection-info" title={t('lanCommentHelp')} aria-label={t('lanCommentHelp')}><Info size={16}/></span>
 </div>{error&&<p role="alert">{t('lanEditError')}</p>}{copied&&<p role="status">{t('lanCopiedSnapshot')}</p>}
 {rows.length>0&&<div className="table-scroll tool-table"><table><thead><tr><th scope="col">{t('lanSelection')}</th>{columns.map(column=><th key={column} scope="col">{t(lanColumnLabels[column])}</th>)}</tr></thead><tbody>{rows.map(row=><tr key={row.device.id} aria-selected={inventory.selectedId===row.device.id} className={inventory.selectedId===row.device.id?'selected-record':''}>
 <td><input type="checkbox" aria-label={`${t('lanSelection')}: ${row.device.address}`} checked={checked.includes(row.device.id)} disabled={busy} onChange={()=>toggle(row.device.id)}/></td>{columns.map(column=><td key={column}>{cell(row,column)}</td>)}</tr>)}</tbody></table></div>}
 </>;
}

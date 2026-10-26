import {useState} from 'react';
import {Tag,MessageSquare,Check,RotateCcw,Info} from 'lucide-react';
import {editServiceNote} from '../core/store';
import {emptyServiceNote,serviceNoteWords,validServiceNote,type ServiceNote,type ServiceNoteEdit} from '../core/service-notes';
import type {Locale} from '../core/types';
import './ServiceNoteEditor.css';
export function ServiceNoteEditor({operation,port,note=emptyServiceNote,locale,disabled,onSave=editServiceNote}:{operation:number;port:number;note?:ServiceNote;locale:Locale;disabled:boolean;onSave?:(operation:number,command:ServiceNoteEdit)=>Promise<void>}){
 const w=serviceNoteWords[locale];
 const [draft,setDraft]=useState<ServiceNote|null>(null),[base,setBase]=useState(note),[pending,setPending]=useState(false),[error,setError]=useState(false),[saved,setSaved]=useState(false);
 const value=draft??note,conflict=!!draft&&base.revision!==note.revision,valid=validServiceNote(value.label,value.comment),dirty=!!draft&&(value.label!==base.label||value.comment!==base.comment);
 function change(field:'label'|'comment',text:string){if(!draft)setBase(note);setDraft({...value,[field]:text});setSaved(false);setError(false);}
 function reload(){setDraft(null);setBase(note);setSaved(false);setError(false);}
 async function save(){if(!dirty||conflict||!valid)return;setPending(true);setError(false);try{await onSave(operation,{port,expectedRevision:base.revision,label:value.label,comment:value.comment});setDraft(null);setSaved(true);}catch{setError(true);}finally{setPending(false);}}
 return <div className="service-note-editor"><div className="service-note-head"><Tag size={16}/><strong>{w.label}</strong><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={15}/></span><span role="status">{saved&&!draft?w.saved:''}</span></div><input maxLength={320} aria-label={w.label} value={value.label} placeholder={w.label} disabled={disabled||pending} onChange={e=>change('label',e.target.value)} aria-invalid={!valid}/><label><MessageSquare size={15}/>{w.comment}<small>{Array.from(value.comment).length}/500</small></label><textarea maxLength={2000} aria-label={w.comment} value={value.comment} placeholder={w.comment} rows={3} disabled={disabled||pending} onChange={e=>change('comment',e.target.value)} aria-invalid={!valid}/><div className="service-note-actions"><button type="button" className="icon-button" aria-label={w.save} title={w.save} disabled={disabled||pending||!valid||!dirty||conflict} onClick={()=>void save()}><Check size={18}/></button><button type="button" className="icon-button" aria-label={w.reload} title={w.reload} disabled={pending||(!draft&&!error)} onClick={reload}><RotateCcw size={17}/></button>{!valid&&<span role="alert">{w.invalid}</span>}{conflict&&<span role="alert">{w.conflict}</span>}{error&&<span role="alert">{w.error}</span>}</div></div>;
}

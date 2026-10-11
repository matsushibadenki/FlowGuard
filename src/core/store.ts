import {useSyncExternalStore} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {Command, Connection, Document, Snapshot} from './types';
import {validRules} from './rules';
export const native=isTauri();
const storageKey='flowguard-preview-v1';
const listeners=new Set<()=>void>();
let snapshot:Snapshot|null=null;
let startupError:string|null=null;
const now=()=>Math.floor(Date.now()/1000);
export function sampleConnections():Connection[]{
 return [['Safari','apple.com',443],['Mail','icloud.com',993],['Dropbox','dropbox.com',443],['Terminal','192.168.1.12',22]].map(([app,destination,port],i)=>({id:`demo-${i}`,app:String(app),appKey:`/Applications/${app}.app`,pid:0,local:'127.0.0.1',destination:String(destination),port:Number(port),protocol:'TCP',direction:'outbound',observedAt:now()-i*60}));
}
function emit(value:Snapshot){if(snapshot&&value.revision<snapshot.revision)return;snapshot=value;for(const l of listeners)l();}
function preview(){
 let document:Document={rules:[],preferences:{locale:'ja',view:'map',panel:'right',mode:'demo'}};
 try {const stored=JSON.parse(localStorage.getItem(storageKey)||'null');if(stored&&validRules(stored.rules)) {document.rules=stored.rules;if(['ja','en','zh-CN'].includes(stored.preferences?.locale))document.preferences.locale=stored.preferences.locale;}}catch{/* Corrupt preview storage is isolated from desktop documents. */}
 const connections=sampleConnections();emit({revision:0,document,connections,history:connections,selectedId:connections[0].id,firewall:{enabled:null,enforcement:'unavailable',observer:'demo',error:null},lastObserved:null,layout:connections.map((c,i)=>({id:c.id,y:72+i*100}))});
}
let initPromise:Promise<void>|undefined;
export function initialize(){
 if(initPromise)return initPromise;
 initPromise=(async()=>{try{if(native){await listen<Snapshot>('state-changed',event=>emit(event.payload));emit(await invoke<Snapshot>('get_snapshot'));}else{preview();window.addEventListener('storage',e=>{if(e.key!==storageKey||!e.newValue||!snapshot)return;try{const doc=JSON.parse(e.newValue);if(doc&&validRules(doc.rules)&&['ja','en','zh-CN'].includes(doc.preferences?.locale)&&['map','apps','rules','history','settings'].includes(doc.preferences?.view)&&['left','right'].includes(doc.preferences?.panel)&&doc.preferences?.mode==='demo')emit({...snapshot,revision:snapshot.revision+1,document:doc});}catch{/* Ignore malformed preview storage events. */}});}}catch(e){startupError=String(e);for(const l of listeners)l();}})();return initPromise;
}
export function useSnapshot(){return useSyncExternalStore(fn=>{listeners.add(fn);return()=>listeners.delete(fn);},()=>snapshot);}
export function useStartupError(){return useSyncExternalStore(fn=>{listeners.add(fn);return()=>listeners.delete(fn);},()=>startupError);}
export async function dispatch(command:Command){
 if(native){emit(await invoke<Snapshot>('dispatch',{command}));return;}
 if(!snapshot)throw Error('not_ready');
 const next:Snapshot=structuredClone(snapshot);const doc=next.document;
 switch(command.type){
 case 'select':if(!next.connections.some(c=>c.id===command.id))throw Error('connection_expired');next.selectedId=command.id;break;
 case 'setView':doc.preferences.view=command.view;break;
 case 'setLocale':doc.preferences.locale=command.locale;break;
 case 'setMode':if(command.mode==='live')throw Error('macos_required');break;
 case 'setPanel':doc.preferences.panel=command.panel;break;
 case 'saveRule':{
  const c=next.connections.find(c=>c.id===command.connectionId);if(!c)throw Error('connection_expired');
  const id=`demo:${c.appKey}:${c.destination}:${c.port}:${command.direction}`;
  const rule={id,appKey:c.appKey,app:c.app,destination:c.destination,port:c.port,direction:command.direction,action:command.action,updatedAt:now(),source:'demo' as const};
  if(!validRules([rule]))throw Error('invalid_rule');const index=doc.rules.findIndex(r=>r.id===id);if(index>=0)doc.rules[index]=rule;else doc.rules.push(rule);break;
 }
 case 'deleteRule':doc.rules=doc.rules.filter(r=>r.id!==command.id);break;
 case 'restoreRules':if(!validRules(command.rules))throw Error('invalid_backup');doc.rules=command.rules;break;
 }
 localStorage.setItem(storageKey,JSON.stringify(doc));next.revision++;emit(next);
}
export async function floatPanel(){if(native){emit(await invoke<Snapshot>('float_panel'));}else{throw Error('desktop_required');}}
export async function dockPanel(panel:'left'|'right'){if(native)emit(await invoke<Snapshot>('dock_panel',{panel}));else await dispatch({type:'setPanel',panel});}

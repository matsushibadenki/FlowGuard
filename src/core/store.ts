import {useSyncExternalStore} from 'react';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import type {Command, Connection, Document, Rule, Snapshot} from './types';
import {validRules} from './rules';
import {defaultExtension} from './enforcement';
import {groupApplications} from './applications';
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
 const connections=sampleConnections();emit({inventory:{generation:0,localAddress:"",netmask:"",status:"first",baselineAt:null,rows:[],selectedId:null,panel:"docked"},discovery:{nameMode:"none",namesCompleted:0,namesTotal:0,stage:"idle",interface:"",range:"",active:false,completed:0,total:0,devices:[],errors:[],observedAt:null},network:{addresses:[],observedAt:null,error:null},revision:0,extension:{...defaultExtension},document,connections,applications:groupApplications(connections),history:connections,selectedId:connections[0].id,firewall:{enabled:null,enforcement:'unavailable',observer:'demo',error:null},lastObserved:null,layout:connections.map((c,i)=>({id:c.id,y:72+i*100}))});
}
let initPromise:Promise<void>|undefined;
export function initialize(){
 if(initPromise)return initPromise;
 initPromise=(async()=>{try{if(native){await listen<Snapshot>('state-changed',event=>emit(event.payload));emit(await invoke<Snapshot>('get_snapshot'));}else{preview();window.addEventListener('storage',e=>{if(e.key!==storageKey||!e.newValue||!snapshot)return;try{const doc=JSON.parse(e.newValue);if(doc&&validRules(doc.rules)&&['ja','en','zh-CN'].includes(doc.preferences?.locale)&&['map','apps','rules','history','settings','network','servers','diagnostics'].includes(doc.preferences?.view)&&['left','right'].includes(doc.preferences?.panel)&&doc.preferences?.mode==='demo')emit({...snapshot,revision:snapshot.revision+1,document:doc});}catch{/* Ignore malformed preview storage events. */}});}}catch(e){startupError=String(e);for(const l of listeners)l();}})();return initPromise;
}
export function useSnapshot(){return useSyncExternalStore(fn=>{listeners.add(fn);return()=>listeners.delete(fn);},()=>snapshot);}
export function useStartupError(){return useSyncExternalStore(fn=>{listeners.add(fn);return()=>listeners.delete(fn);},()=>startupError);}
export async function dispatch(command:Command, expectedRules:Rule[]=snapshot?.document.rules??[]){
 if(native){const next=await invoke<Snapshot>('dispatch',{command,expectedRules});emit(next);return next;}
 if(!snapshot)throw Error('not_ready');
 if(['saveRule','deleteRule','restoreRules'].includes(command.type)&&JSON.stringify(expectedRules)!==JSON.stringify(snapshot.document.rules))throw Error('edit_conflict');
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
 localStorage.setItem(storageKey,JSON.stringify(doc));next.revision++;emit(next);return next;
}
export async function floatPanel(){if(native){emit(await invoke<Snapshot>('float_panel'));}else{throw Error('desktop_required');}}
export async function dockPanel(panel:'left'|'right'){if(native)emit(await invoke<Snapshot>('dock_panel',{panel}));else await dispatch({type:'setPanel',panel});}

export interface ExtensionPackageStatus {packaged:boolean;bridgeAvailable:boolean;status:string}
export async function extensionPackageStatus():Promise<ExtensionPackageStatus>{return native?invoke<ExtensionPackageStatus>('extension_package_status'):{packaged:false,bridgeAvailable:false,status:'not_packaged'};}

export async function activateExtension(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('activate_extension'));}
export async function applyExtensionRules(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('sync_extension_rules'));}
export async function refreshExtension(){if(!native)return;emit(await invoke<Snapshot>('refresh_extension'));}

export async function disableExtension(remove=false){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('disable_extension',{remove}));}

export async function startLanDiscovery(interfaceName:string,address:string,active:boolean,nameMode:import('./types').NameMode='none') {
 if(!native)throw Error('desktop_required');
 emit(await invoke<Snapshot>('start_lan_discovery',{interface:interfaceName,address,active,nameMode}));
}
export async function stopLanDiscovery() {
 if(!native)throw Error('desktop_required');
 emit(await invoke<Snapshot>('stop_lan_discovery'));
}

export async function selectLanDevice(id:string){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('select_lan_device',{id}));}
export async function floatLanDetails(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('float_lan_details'));}
export async function dockLanDetails(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('dock_lan_details'));}
export async function lanCsv(generation:number):Promise<string>{if(!native)throw Error('desktop_required');return invoke<string>('lan_csv',{generation});}

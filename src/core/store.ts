import {validHistoryRetention,retainedHistory} from './history';
import {version as packageVersion} from '../../package.json';
import {lanColumns,validLanColumns,defaultLanWidth,type LanEdit} from './lan-workbench';
// パス: src/core/store.ts
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
 let document:Document={rules:[],preferences:{updateChannel:'stable',historyRetention:'session',protectionGuideSeen:false,unknownNotifications:false,askNotifications:false,askTimeoutAction:'allow',locale:'ja',view:'map',panel:'right',mode:'demo',autoUpdates:true}};
 try {const stored=JSON.parse(localStorage.getItem(storageKey)||'null');if(stored&&validRules(stored.rules)) {document.rules=stored.rules;if(['stable','beta'].includes(stored.preferences?.updateChannel))document.preferences.updateChannel=stored.preferences.updateChannel;if(validHistoryRetention(stored.preferences?.historyRetention))document.preferences.historyRetention=stored.preferences.historyRetention;if(typeof stored.preferences?.protectionGuideSeen==='boolean')document.preferences.protectionGuideSeen=stored.preferences.protectionGuideSeen;if(typeof stored.preferences?.unknownNotifications==='boolean')document.preferences.unknownNotifications=stored.preferences.unknownNotifications;if(typeof stored.preferences?.askNotifications==='boolean')document.preferences.askNotifications=stored.preferences.askNotifications;if(['allow','block'].includes(stored.preferences?.askTimeoutAction))document.preferences.askTimeoutAction=stored.preferences.askTimeoutAction;if(['ja','en','zh-CN'].includes(stored.preferences?.locale))document.preferences.locale=stored.preferences.locale;if(typeof stored.preferences?.autoUpdates==='boolean')document.preferences.autoUpdates=stored.preferences.autoUpdates;}}catch{/* Corrupt preview storage is isolated from desktop documents. */}
 const connections=sampleConnections();emit({asks:{requests:[],outcomes:[],busyIds:[],pollRevision:0,error:null,lastAnswer:null},security:{flows:[],events:[],routerCapabilities:{deviceDiscovery:"unknown",trafficCounters:"unknown",flowExport:"unknown",securityLogs:"unknown",firewallRead:"unknown",firewallWrite:"unknown",deviceQuarantine:"unknown",configBackup:"unknown"}},updates:{stage:"idle",operation:0,currentVersion:packageVersion,version:null,notes:null,checkedAt:null,downloaded:0,total:null,error:null,development:true},services:{operation:0,portOperation:0,generation:0,target:"",sourceAddress:"",interface:"",family:"",mode:"",stage:"",total:0,completed:0,rows:[],error:null,selectedPort:null},portsFocus:0,ports:{family:"",sourceNetmask:"",comparison:{status:"",baselineOperation:null,rows:[]},operation:0,sourceAddress:"",requestedPorts:[],selection:"",stage:"",target:"",interface:"",generation:0,completed:0,total:0,rows:[],error:null},inventory:{generation:0,localAddress:"",netmask:"",status:"first",baselineAt:null,rows:[],selectedId:null,panel:"docked"},discovery:{nameMode:"none",namesCompleted:0,namesTotal:0,stage:"idle",interface:"",range:"",active:false,completed:0,total:0,devices:[],errors:[],observedAt:null},network:{addresses:[],observedAt:null,error:null},revision:0,extension:{...defaultExtension},document,connections,applications:groupApplications(connections),history:retainedHistory(connections,document.preferences.historyRetention??'session',now()),selectedId:connections[0].id,firewall:{enabled:null,enforcement:'unavailable',observer:'demo',error:null},lastObserved:null,layout:connections.map((c,i)=>({id:c.id,y:72+i*100}))});
}
let initPromise:Promise<void>|undefined;
export function initialize(){
 if(initPromise)return initPromise;
 initPromise=(async()=>{try{if(native){await listen<Snapshot>('state-changed',event=>emit(event.payload));emit(await invoke<Snapshot>('get_snapshot'));}else{preview();window.addEventListener('storage',e=>{if(e.key!==storageKey||!e.newValue||!snapshot)return;try{const doc=JSON.parse(e.newValue);if(doc&&validRules(doc.rules)&&['ja','en','zh-CN'].includes(doc.preferences?.locale)&&['map','apps','rules','history','settings','network','servers','diagnostics'].includes(doc.preferences?.view)&&['left','right'].includes(doc.preferences?.panel)&&doc.preferences?.mode==='demo'&&(doc.preferences.historyRetention===undefined||validHistoryRetention(doc.preferences.historyRetention)))emit({...snapshot,revision:snapshot.revision+1,document:doc,history:retainedHistory(snapshot.history,doc.preferences.historyRetention??'session',now())});}catch{/* Ignore malformed preview storage events. */}});}}catch(e){startupError=String(e);for(const l of listeners)l();}})();return initPromise;
}
export function useSnapshot(){return useSyncExternalStore(fn=>{listeners.add(fn);return()=>listeners.delete(fn);},()=>snapshot);}
export function useStartupError(){return useSyncExternalStore(fn=>{listeners.add(fn);return()=>listeners.delete(fn);},()=>startupError);}
export async function dispatch(command:Command, expectedRules:Rule[]=snapshot?.document.rules??[]){
 if(native){const next=await invoke<Snapshot>('dispatch',{command,expectedRules});emit(next);return next;}
 if(!snapshot)throw Error('not_ready');
 if(snapshot.updates.stage==='installing')throw Error('update_installing');
 if(['setSystemException','setAskTimeoutAction','saveRule','migrateRule','deleteRule','restoreRules'].includes(command.type)&&JSON.stringify(expectedRules)!==JSON.stringify(snapshot.document.rules))throw Error('edit_conflict');
 const next:Snapshot=structuredClone(snapshot);const doc=next.document;
 switch(command.type){
 case 'setUpdateChannel':if(!['stable','beta'].includes(command.channel)||['checking','downloading','ready','installing'].includes(next.updates.stage))throw Error('updater_busy');doc.preferences.updateChannel=command.channel;next.updates.channel=command.channel;next.updates.stage='idle';break;
 case 'setHistoryRetention':if(!validHistoryRetention(command.retention))throw Error('invalid_retention');doc.preferences.historyRetention=command.retention;next.history=retainedHistory(next.history,command.retention,now());break;
 case 'acknowledgeProtectionGuide':doc.preferences.protectionGuideSeen=true;break;
 case 'setUnknownNotifications':doc.preferences.unknownNotifications=command.enabled;break;
 case 'setAskNotifications':doc.preferences.askNotifications=command.enabled;break;
 case 'setSystemException':throw Error('desktop_required');
 case 'setAskTimeoutAction':doc.preferences.askTimeoutAction=command.action;for(const r of doc.rules){if(r.action==='ask'){r.timeoutAction=command.action;r.updatedAt=now();}}break;
 case 'select':if(!next.connections.some(c=>c.id===command.id))throw Error('connection_expired');next.selectedId=command.id;break;
 case 'setAutoUpdates':doc.preferences.autoUpdates=command.enabled;break;
  case 'setView':doc.preferences.view=command.view;break;
 case 'setLocale':doc.preferences.locale=command.locale;break;
 case 'setMode':if(command.mode==='live')throw Error('macos_required');break;
 case 'setPanel':doc.preferences.panel=command.panel;break;
 case 'saveRule':{
  const c=next.connections.find(c=>c.id===command.connectionId);if(!c)throw Error('connection_expired');
  const id=`demo:${c.appKey}:${c.destination}:${c.port}:${command.direction}`;
  const rule={timeoutAction:doc.preferences.askTimeoutAction??'allow',id,appKey:c.appKey,app:c.app,destination:c.destination,port:c.port,direction:command.direction,action:command.action,updatedAt:now(),source:'demo' as const};
  if(!validRules([rule]))throw Error('invalid_rule');const index=doc.rules.findIndex(r=>r.id===id);if(index>=0)doc.rules[index]=rule;else doc.rules.push(rule);break;
 }
 case 'migrateRule':throw Error('macos_required');
 case 'deleteRule':doc.rules=doc.rules.filter(r=>r.id!==command.id);break;
 case 'restoreRules':if(!validRules(command.rules))throw Error('invalid_backup');doc.rules=command.rules.map(rule=>({...rule,systemException:false}));break;
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

export async function startLanDiscovery(interfaceName:string,address:string,active:boolean,nameMode:import('./types').NameMode='none',options?:import('./lan-sheet').ScanOptions) {
 if(!native)throw Error('desktop_required');
 emit(await invoke<Snapshot>('start_lan_discovery',{interface:interfaceName,address,active,nameMode,options}));
}
export async function stopLanDiscovery() {
 if(!native)throw Error('desktop_required');
 emit(await invoke<Snapshot>('stop_lan_discovery'));
}

export async function selectLanDevice(id:string){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('select_lan_device',{id}));}
export async function floatLanDetails(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('float_lan_details'));}
export async function dockLanDetails(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('dock_lan_details'));}
export async function lanCsv(generation:number):Promise<string>{if(!native)throw Error('desktop_required');return invoke<string>('lan_csv',{generation});}

export async function startLanPorts(id:string,generation:number,first:number,last:number,preset:import('./types').PortPreset='range',settings?:import('./port-settings').PortScanSettings){if(!native)throw Error("desktop_required");emit(await invoke<Snapshot>("start_lan_ports",{id,generation,first,last,preset,settings}));}
export async function stopLanPorts(){if(!native)throw Error("desktop_required");emit(await invoke<Snapshot>("stop_lan_ports"));}

export async function lanPortsCsv(operation:number):Promise<string>{if(!native)throw Error("desktop_required");return invoke<string>("lan_ports_csv",{operation});}

export async function openLanPorts(id:string,generation:number){if(!native)throw Error("desktop_required");emit(await invoke<Snapshot>("open_lan_ports",{id,generation}));}

export async function startServices(operation:number,mode:import('./types').ServiceMode,hostname=''){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('start_services',{operation,mode,hostname}));}
export async function stopServices(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('stop_services'));}
export async function servicesCsv(operation:number):Promise<string>{if(!native)throw Error('desktop_required');return invoke<string>('services_csv',{operation});}
export async function selectService(operation:number,port:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('select_service',{operation,port}));}

export async function checkUpdate(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('check_update'));}
export async function downloadUpdate(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('download_update'));}
export async function cancelUpdate(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('cancel_update'));}
export async function installUpdate(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('install_update'));}

export async function answerAsk(request:import('./types').AskRequest,action:'allow'|'block') {
 if(!native)throw Error('macos_required');
 const value=await invoke<Snapshot>('answer_ask',{requestId:request.requestId,instanceId:request.instanceId,policyId:request.policyId,action});emit(value);return value;
}

export async function notificationPermission(request=false):Promise<string>{if(!native)return 'unavailable';return invoke<string>('notification_permission',{request});}

export async function editLanInventory(generation:number,command:LanEdit):Promise<Snapshot>{
 if(native){const next=await invoke<Snapshot>('edit_lan_inventory',{generation,command});emit(next);return next;}
 if(!snapshot||snapshot.inventory.generation!==generation||snapshot.inventory.status==='running')throw Error('lan_stale');
 const inventory={...snapshot.inventory,checkedIds:[...(snapshot.inventory.checkedIds??[])],comments:{...snapshot.inventory.comments},labels:{...snapshot.inventory.labels},widths:{...snapshot.inventory.widths},columns:[...(snapshot.inventory.columns??lanColumns)]};
 if(command.type==='selection'){
  if(JSON.stringify(command.expected)!==JSON.stringify(inventory.checkedIds))throw Error('lan_conflict');
  if(command.ids.length>2048||command.ids.some(id=>!inventory.rows.some(r=>r.device.id===id)))throw Error('lan_stale');
  inventory.checkedIds=[...new Set(command.ids)].sort();
 }else if(command.type==='comment'){
  if(!inventory.rows.some(r=>r.device.id===command.id))throw Error('lan_stale');
  if((inventory.comments[command.id]??'')!==command.expected)throw Error('lan_conflict');
  if([...command.value].length>500||new TextEncoder().encode(command.value).length>2000||/[\x00-\x09\x0b-\x1f\x7f]/.test(command.value))throw Error('lan_comment_invalid');
  if(command.value)inventory.comments[command.id]=command.value;else delete inventory.comments[command.id];
 }else if(command.type==='width'){
  if(!lanColumns.includes(command.column as typeof lanColumns[number])||!Number.isInteger(command.value)||command.value<80||command.value>480)throw Error('lan_width_invalid');
  if((inventory.widths[command.column]??defaultLanWidth(command.column))!==command.expected)throw Error('lan_conflict');inventory.widths[command.column]=command.value;
 }else if(command.type==='label'){
  if(!inventory.rows.some(r=>r.device.id===command.id)||[...command.value].length>80||/[\x00-\x1f\x7f]/.test(command.value))throw Error('lan_label_invalid');
  if((inventory.labels[command.id]??'')!==command.expected)throw Error('lan_conflict');if(command.value)inventory.labels[command.id]=command.value;else delete inventory.labels[command.id];
 }else if(command.type==='archive'){throw Error('desktop_required');
 }else{
  if(JSON.stringify(command.expected)!==JSON.stringify(inventory.columns))throw Error('lan_conflict');
  if(!validLanColumns(command.values))throw Error('lan_columns_invalid');inventory.columns=command.values;
 }
 const next={...snapshot,inventory,revision:snapshot.revision+1};emit(next);return next;
}
export async function copyLanSelection(generation:number):Promise<void>{if(!native)throw Error('desktop_required');return invoke('copy_lan_selection',{generation});}

export async function startSyslog(config:import('./types').SyslogConfig){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('start_syslog',{config}));}
export async function stopSyslog(operation:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('stop_syslog',{operation}));}
export async function clearSyslog(operation:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('clear_syslog',{operation}));}
export async function selectSyslog(operation:number,id:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('select_syslog',{operation,id}));}

export async function controlFirewall(action:import('./firewall-control').FirewallAction,expected:boolean|null,acknowledged:boolean){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('control_firewall',{action,expected,acknowledged}));}

export async function setDestinationNames(enabled:boolean,operation:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('set_destination_names',{enabled,operation}));}
export async function lookupDestinationName(connectionId:string,address:string,operation:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('lookup_destination_name',{connectionId,address,operation}));}
export async function clearDestinationNames(operation:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('clear_destination_names',{operation}));}

export async function diagnosticAction(action:string,report=0,finding:string|null=null,serviceOperation=0,disposition:import('./types').DiagnosticDisposition|null=null,reason:string|null=null){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('diagnostic_action',{action,report,finding,serviceOperation,disposition,reason}));}
export async function diagnosticExport(report:number,format:'csv'|'html'){if(!native)throw Error('desktop_required');return invoke<string>('diagnostic_export',{report,format});}

export async function setFlowNames(enabled:boolean,search:boolean,operation:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('set_flow_names',{enabled,search,operation}));}
export async function collectFlowNames(operation:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('collect_flow_names',{operation}));}

export async function clearUpdateCache(){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('clear_update_download_cache'));}

export async function temporaryAction(action:string,connectionId:string|null=null,id:string|null=null,direction:string|null=null,peerEvidence:string|null=null,seconds=60,confirmed=false){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('temporary_protection_action',{action,connectionId,id,direction,peerEvidence,seconds,confirmed}));}

export async function selectPort(operation:number,port:number){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('select_port',{operation,port}));}

/** Native auxiliary query results share the same Rust snapshot as every window. */
export function acceptSnapshot(value:Snapshot){emit(value);}

export async function getLanArchive(generation:number,observedAt:number):Promise<import('./lan-sheet').LanArchive>{if(!native)throw Error('desktop_required');return invoke('get_lan_archive',{generation,observedAt});}

export async function setPortLayout(expected:import('./port-layout').PortLayout,value:import('./port-layout').PortLayout){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('set_port_layout',{expected,value}));}

export async function positionPortDetails(panel:import('./port-panel').PortPanel){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('position_port_details',{panel}));}

export async function editServiceNote(operation:number,command:import('./service-notes').ServiceNoteEdit){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('edit_service_note',{operation,command}));}

export async function positionServiceDetails(panel:import('./port-panel').PortPanel){if(!native)throw Error('desktop_required');emit(await invoke<Snapshot>('position_service_details',{panel}));}

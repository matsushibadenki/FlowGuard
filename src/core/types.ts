// パス: src/core/types.ts
export type Locale = 'ja' | 'en' | 'zh-CN';
export type View = 'map' | 'apps' | 'rules' | 'history' | 'settings' | 'network' | 'servers' | 'diagnostics';
export type Action = 'allow' | 'block' | 'ask';
export type Direction = 'outbound' | 'inbound' | 'both';
export interface SigningIdentity {authority:'apple'|'developer';teamId:string;identifier:string}
export interface Connection {identity?:SigningIdentity|null;id:string;app:string;appKey:string;pid:number;local:string;destination:string;port:number;protocol:string;direction:string;observedAt:number}
export interface Rule {systemException?:boolean;timeoutAction?:'allow'|'block';identity?:SigningIdentity|null;id:string;appKey:string;app:string;destination:string;port:number;direction:Direction;action:Action;updatedAt:number;source:'live'|'demo'}
export interface Document {rules:Rule[];preferences:{askNotifications?:boolean;askTimeoutAction?:'allow'|'block';locale:Locale;view:View;panel:'left'|'right'|'floating';mode:'live'|'demo';autoUpdates:boolean}}
export interface ApplicationGroup {id:string;app:string;appKey:string;connectionIds:string[];processIds:number[];destinations:string[]}
export interface ExtensionStatus {stage:string;packaged:boolean;runtimeAvailable:boolean;signed:boolean;busy:boolean;instanceId:string|null;providerRevision:number|null;appliedRuleIds:string[];error:string|null}
export interface NetworkSnapshot {addresses:{interface:string;address:string;family:string;netmask:string;up:boolean}[];observedAt:number|null;error:string|null}
export type NameMode = 'none' | 'mdns' | 'system';
export interface NameObservation {value:string|null;source:string|null;status:string;observedAt:number|null}
export interface LanDevice {name:NameObservation;id:string;interface:string;address:string;family:string;mac:string|null;source:string;response:string;observedAt:number}
export interface DiscoverySnapshot {nameMode:NameMode;namesCompleted:number;namesTotal:number;stage:string;interface:string;range:string;active:boolean;completed:number;total:number;devices:LanDevice[];errors:string[];observedAt:number|null}
export type PortPreset = 'common' | 'web' | 'admin' | 'range';
export interface PortRow {port:number;status:string;elapsedMs:number;observedAt:number}
export interface PortComparison {status:string;baselineOperation:number|null;rows:{port:number;kind:string;previous:PortRow|null}[]}
export interface PortSnapshot {family:string;sourceNetmask:string;comparison:PortComparison;operation:number;sourceAddress:string;requestedPorts:number[];selection:PortPreset|'';stage:string;target:string;interface:string;generation:number;completed:number;total:number;rows:PortRow[];error:string|null}
export interface Snapshot {asks?:AskSnapshot;security:SecuritySnapshot;updates:UpdateSnapshot;services:ServiceSnapshot;portsFocus:number;ports:PortSnapshot;inventory:InventorySnapshot;discovery:DiscoverySnapshot;network:NetworkSnapshot;revision:number;extension:ExtensionStatus;document:Document;connections:Connection[];applications:ApplicationGroup[];history:Connection[];selectedId:string|null;firewall:{enabled:boolean|null;enforcement:string;observer:string;error:string|null};lastObserved:number|null;layout:{id:string;y:number}[]}
export type Command = {type:'setAskNotifications';enabled:boolean}| {type:'setSystemException';id:string;enabled:boolean}| {type:'setAskTimeoutAction';action:'allow'|'block'}|{type:'setAutoUpdates';enabled:boolean}|{type:'select';id:string}|{type:'setView';view:View}|{type:'setLocale';locale:Locale}|{type:'setMode';mode:'demo'|'live'}|{type:'setPanel';panel:'left'|'right'|'floating'}|{type:'saveRule';connectionId:string;action:Action;direction:Direction}|{type:'migrateRule';id:string;connectionId:string}|{type:'deleteRule';id:string}|{type:'restoreRules';rules:Rule[]};
export interface DeviceChange {device:LanDevice;previous:LanDevice|null;kind:string;fields:string[]}
export interface InventorySnapshot {generation:number;localAddress:string;netmask:string;status:string;baselineAt:number|null;rows:DeviceChange[];selectedId:string|null;panel:'docked'|'floating'}

export type ServiceMode='auto'|'http'|'https'|'ssh'|'smtp';
export interface ServiceRow {port:number;candidate:string;service:string|null;status:string;method:string;product:string|null;response:string|null;tls:string|null;observedAt:number}
export interface ServiceSnapshot {operation:number;portOperation:number;generation:number;target:string;sourceAddress:string;interface:string;family:string;mode:string;stage:string;total:number;completed:number;rows:ServiceRow[];error:string|null;selectedPort:number|null}

export interface UpdateSnapshot {stage:string;operation:number;currentVersion:string;version:string|null;notes:string|null;checkedAt:number|null;downloaded:number;total:number|null;error:string|null;development:boolean}

export type SecuritySource = 'mac_socket' | 'router' | 'demo';
export type Capability = 'available' | 'unavailable' | 'unknown';
export interface RouterCapabilities {deviceDiscovery:Capability;trafficCounters:Capability;flowExport:Capability;securityLogs:Capability;firewallRead:Capability;firewallWrite:Capability;deviceQuarantine:Capability;configBackup:Capability}
export interface SecurityFlow {id:string;source:SecuritySource;sourceId:string;localEndpoint:string;remoteAddress:string;remotePort:number;protocol:string;direction:string;appKey:string|null;pid:number|null;firstObservedAt:number;sampledAt:number|null;bytesSent:number|null;bytesReceived:number|null;counterBasis:string}
export interface SecurityEvent {id:string;source:SecuritySource;sourceId:string;observedAt:number;kind:string;evidenceFlowIds:string[]}
export interface SecuritySnapshot {flows:SecurityFlow[];events:SecurityEvent[];routerCapabilities:RouterCapabilities}

export interface AskRequest {requestId:string;instanceId:string;policyId:string;ruleId:string;identity:SigningIdentity;app:string;destination:string;port:number;direction:'inbound'|'outbound';timeoutAction:'allow'|'block';remainingSeconds:number}
export interface AskOutcome {request:AskRequest;action:'allow'|'block';reason:'answered'|'timed_out'|'policy_changed'|'provider_stopped'|'capacity'|'transport_unavailable'}
export interface AskSnapshot {requests:AskRequest[];outcomes:AskOutcome[];busyIds:string[];pollRevision:number;error:string|null;lastAnswer:string|null}

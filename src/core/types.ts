// パス: src/core/types.ts
export type Locale = 'ja' | 'en' | 'zh-CN';
export type View = 'map' | 'apps' | 'rules' | 'history' | 'settings' | 'network' | 'servers' | 'diagnostics';
export type Action = 'allow' | 'block' | 'ask';
export type Direction = 'outbound' | 'inbound' | 'both';
export interface SigningIdentity {authority:'apple'|'developer';teamId:string;identifier:string}
export interface Connection {identity?:SigningIdentity|null;id:string;app:string;appKey:string;pid:number;local:string;destination:string;port:number;protocol:string;direction:string;observedAt:number}
export interface Rule {transport?:'TCP';systemException?:boolean;timeoutAction?:'allow'|'block';identity?:SigningIdentity|null;id:string;appKey:string;app:string;destination:string;port:number;direction:Direction;action:Action;updatedAt:number;source:'live'|'demo'}
export interface Document {rules:Rule[];preferences:{updateChannel?:'stable'|'beta';historyRetention?:import('./history').HistoryRetention;protectionGuideSeen?:boolean;unknownNotifications?:boolean;askNotifications?:boolean;askTimeoutAction?:'allow'|'block';locale:Locale;view:View;panel:'left'|'right'|'floating';mode:'live'|'demo';autoUpdates:boolean}}
export interface ApplicationGroup {id:string;app:string;appKey:string;connectionIds:string[];processIds:number[];destinations:string[]}
export interface ExtensionStatus {stage:string;packaged:boolean;runtimeAvailable:boolean;signed:boolean;busy:boolean;instanceId:string|null;providerRevision:number|null;appliedRuleIds:string[];error:string|null}
export interface NetworkSnapshot {addresses:{interface:string;address:string;family:string;netmask:string;up:boolean}[];observedAt:number|null;error:string|null}
export type NameMode = 'none' | 'mdns' | 'system';
export interface NameObservation {values?:string[];value:string|null;source:string|null;status:string;observedAt:number|null}
export interface LanDevice {name:NameObservation;id:string;interface:string;address:string;family:string;mac:string|null;source:string;response:string;observedAt:number}
export interface DiscoverySnapshot {metadata?:Record<string,import('./lan-sheet').LanMetadata>;context?:{gateway:string|null;ssid:string|null};options?:import('./lan-sheet').ScanOptions;responseMs?:Record<string,number>;nameMode:NameMode;namesCompleted:number;namesTotal:number;stage:string;interface:string;range:string;active:boolean;completed:number;total:number;devices:LanDevice[];errors:string[];observedAt:number|null}
export type PortPreset = 'common' | 'web' | 'admin' | 'range';
export interface PortRow {port:number;status:string;elapsedMs:number;observedAt:number}
export interface PortComparison {status:string;baselineOperation:number|null;rows:{port:number;kind:string;previous:PortRow|null}[]}
export interface PortSnapshot {panel?:import('./port-panel').PortPanel;layout?:import('./port-layout').PortLayout;settings?:import('./port-settings').PortScanSettings;budgetMs?:number;selectedPort?:number|null;family:string;sourceNetmask:string;comparison:PortComparison;operation:number;sourceAddress:string;requestedPorts:number[];selection:PortPreset|'';stage:string;target:string;interface:string;generation:number;completed:number;total:number;rows:PortRow[];error:string|null}
export interface FirewallControlSnapshot {operation:number;busy:boolean;service:string;enabled:boolean|null;error:string|null}
export interface DestinationNameRow {address:string;value:string|null;source:'system_resolver';observedAt:number;status:string;stale:boolean;trust:'unverified_name'}
export interface DestinationNamesSnapshot {enabled:boolean;operation:number;busy:boolean;cancelling:boolean;activeAddress:string|null;cooldownSeconds:number;rows:DestinationNameRow[]}
export interface Snapshot {gpuPanelOwner?:'main'|'gpu-map';gpuFloatOperation?:number;gpuDockOperation?:number;temporaryProtection?:TemporarySnapshot;flowNames?:FlowNamesSnapshot;gpuFilter?:import('./gpu-panel').GpuFilter;diagnostics?:DiagnosticSnapshot;destinationNames?:DestinationNamesSnapshot;firewallControl?:FirewallControlSnapshot;syslog?:SyslogSnapshot;asks?:AskSnapshot;security:SecuritySnapshot;updates:UpdateSnapshot;services:ServiceSnapshot;portsFocus:number;ports:PortSnapshot;inventory:InventorySnapshot;discovery:DiscoverySnapshot;network:NetworkSnapshot;revision:number;extension:ExtensionStatus;document:Document;connections:Connection[];applications:ApplicationGroup[];history:Connection[];selectedId:string|null;firewall:{enabled:boolean|null;enforcement:string;observer:string;error:string|null};lastObserved:number|null;layout:{id:string;y:number}[]}
export type Command = {type:'setUpdateChannel';channel:'stable'|'beta'}|{type:'setHistoryRetention';retention:import('./history').HistoryRetention}|{type:'acknowledgeProtectionGuide'}|{type:'setUnknownNotifications';enabled:boolean}| {type:'setAskNotifications';enabled:boolean}| {type:'setSystemException';id:string;enabled:boolean}| {type:'setAskTimeoutAction';action:'allow'|'block'}|{type:'setAutoUpdates';enabled:boolean}|{type:'select';id:string}|{type:'setView';view:View}|{type:'setLocale';locale:Locale}|{type:'setMode';mode:'demo'|'live'}|{type:'setPanel';panel:'left'|'right'|'floating'}|{type:'saveRule';connectionId:string;action:Action;direction:Direction}|{type:'migrateRule';id:string;connectionId:string}|{type:'deleteRule';id:string}|{type:'restoreRules';rules:Rule[]};
export interface DeviceChange {device:LanDevice;previous:LanDevice|null;kind:string;fields:string[]}
export interface InventorySnapshot {publicInfo?:{enabled:boolean;busy:boolean;operation:number;ip:string|null;organization:string|null;country:string|null;observedAt:number|null;error:boolean;source:string};vendors?:Record<string,{organization:string;prefix:string;source:string;retrievedAt:string;trust:string}>;widths?:Record<string,number>;labels?:Record<string,string>;firstSeen?:Record<string,number>;archives?:{generation:number;observedAt:number;interface:string;localAddress:string;range:string;rowCount:number}[];archiveError?:boolean;candidates?:Record<string,string[]>;checkedIds?:string[];comments?:Record<string,string>;columns?:string[];generation:number;localAddress:string;netmask:string;status:string;baselineAt:number|null;rows:DeviceChange[];selectedId:string|null;panel:'docked'|'floating'}

export type ServiceMode='auto'|'http'|'https'|'ssh'|'smtp';
export interface TlsDetails {serverName:string;verification:string;reason:string;certificate:{issuer:string|null;notBefore:number|null;notAfter:number|null}|null}
export interface ServiceRow {tlsDetails?:TlsDetails|null;port:number;candidate:string;service:string|null;status:string;method:string;product:string|null;response:string|null;tls:string|null;observedAt:number}
export interface ServiceChange {port:number;kind:'changed'|'unchanged'|'uncertain'|'uncompared';fields:string[];previous:ServiceRow|null}
export interface ServiceComparison {status:string;baselineOperation:number|null;baselinePortOperation:number|null;rows:ServiceChange[]}
export interface ServiceSnapshot {comparison?:ServiceComparison;requestedPorts?:number[];sourceNetmask?:string;hostname?:string|null;operation:number;portOperation:number;generation:number;target:string;sourceAddress:string;interface:string;family:string;mode:string;stage:string;total:number;completed:number;rows:ServiceRow[];error:string|null;selectedPort:number|null}

export interface UpdateSnapshot {channel?:'stable'|'beta';history?:{at:number;status:string;from:string;to:string|null}[];historyPersistent?:boolean;stage:string;operation:number;currentVersion:string;version:string|null;notes:string|null;checkedAt:number|null;downloaded:number;total:number|null;error:string|null;development:boolean}

export type SecuritySource = 'mac_socket' | 'router' | 'demo';
export type Capability = 'available' | 'unavailable' | 'unknown';
export interface RouterCapabilities {deviceDiscovery:Capability;trafficCounters:Capability;flowExport:Capability;securityLogs:Capability;firewallRead:Capability;firewallWrite:Capability;deviceQuarantine:Capability;configBackup:Capability}
export interface SecurityFlow {id:string;source:SecuritySource;sourceId:string;localEndpoint:string;remoteAddress:string;remotePort:number;protocol:string;direction:string;appKey:string|null;pid:number|null;firstObservedAt:number;sampledAt:number|null;bytesSent:number|null;bytesReceived:number|null;counterBasis:string}
export interface SecurityEvent {id:string;source:SecuritySource;sourceId:string;observedAt:number;kind:string;evidenceFlowIds:string[]}
export type RouterErrorCode='invalid_config'|'busy'|'timeout'|'cancelled'|'authentication'|'source_mismatch'|'invalid_payload'|'limit'|'replay'|'stale'|'unavailable'|'internal';
export interface RouterSourceStatus {id:string;endpoint:string;location:'lan_before_nat'|'wan_after_nat'|'unknown';stage:'idle'|'collecting'|'ready'|'cancelled'|'failed'|'stale';authentication:'unknown'|'verified'|'unverified';sampledAt:number|null;error:{code:RouterErrorCode;retryable:boolean}|null}
export interface FlowCorrelation {macFlowId:string;routerFlowId:string;basis:'same_tuple_near_sample_candidate_only'}
export interface SecuritySnapshot {routerSources?:RouterSourceStatus[];correlations?:FlowCorrelation[];flows:SecurityFlow[];events:SecurityEvent[];routerCapabilities:RouterCapabilities}

export interface AskRequest {requestId:string;instanceId:string;policyId:string;ruleId:string;identity:SigningIdentity;app:string;destination:string;port:number;direction:'inbound'|'outbound';timeoutAction:'allow'|'block';remainingSeconds:number}
export interface AskOutcome {request:AskRequest;action:'allow'|'block';reason:'answered'|'timed_out'|'policy_changed'|'provider_stopped'|'capacity'|'transport_unavailable'}
export interface AskSnapshot {requests:AskRequest[];outcomes:AskOutcome[];busyIds:string[];pollRevision:number;error:string|null;lastAnswer:string|null}

export interface SyslogConfig {bindAddress:string;port:number;sourceAddress:string;transport:'udp'|'tcp_newline';durationSeconds:number;acknowledgeUnverified:boolean}
export interface SyslogCounts {bytesRead:number;frames:number;accepted:number;rejectedSource:number;invalid:number;oversized:number;rateDropped:number;evicted:number;incomplete:number;connections:number}
export interface SyslogRow {id:number;receivedAt:number;peer:string;priority:number;text:string;sanitized:boolean;trust:'unverified_source_and_content'}
export interface SyslogSnapshot {operation:number;config:SyslogConfig|null;stage:string;boundEndpoint:string|null;counts:SyslogCounts;rows:SyslogRow[];selectedId:number|null;error:string|null}

export type DiagnosticDisposition='open'|'resolved'|'recheck'|'out_of_scope';
export interface DiagnosticFinding{id:string;port:number;category:string;severity:string;certainty:string;evidence:string;recommendation:string;observedAt:number;change:string;disposition:DiagnosticDisposition;reason:string}
export interface DiagnosticReport{id:number;serviceOperation:number;generation:number;target:string;interface:string;createdAt:number;findings:DiagnosticFinding[]}
export interface DiagnosticSnapshot{reports:DiagnosticReport[];selectedReport:number|null;selectedFinding:string|null}

export interface FlowNamesSnapshot{enabled:boolean;operation:number;busy:boolean;searchEnabled:boolean;rows:{connectionId:string;values:{value:string;observedAt:number;source:'network_extension_remote_hostname';trust:'unverified_tuple_time_candidate'}[]}[];error:string|null}

export interface TemporarySnapshot{operation:number;busy:boolean;received:boolean;error:string|null;rows:{id:string;connectionId:string;app:string;local:string;remote:string;interfaceName:string;peerEvidence:string;remainingSeconds:number;received:boolean}[]}

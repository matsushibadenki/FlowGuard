export type Locale = 'ja' | 'en' | 'zh-CN';
export type View = 'map' | 'apps' | 'rules' | 'history' | 'settings';
export type Action = 'allow' | 'block' | 'ask';
export type Direction = 'outbound' | 'inbound' | 'both';
export interface Connection {id:string;app:string;appKey:string;pid:number;local:string;destination:string;port:number;protocol:string;direction:string;observedAt:number}
export interface Rule {id:string;appKey:string;app:string;destination:string;port:number;direction:Direction;action:Action;updatedAt:number;source:'live'|'demo'}
export interface Document {rules:Rule[];preferences:{locale:Locale;view:View;panel:'left'|'right'|'floating';mode:'live'|'demo'}}
export interface ApplicationGroup {id:string;app:string;appKey:string;connectionIds:string[];processIds:number[];destinations:string[]}
export interface ExtensionStatus {stage:string;packaged:boolean;runtimeAvailable:boolean;signed:boolean;busy:boolean;instanceId:string|null;providerRevision:number|null;appliedRuleIds:string[];error:string|null}
export interface Snapshot {revision:number;extension:ExtensionStatus;document:Document;connections:Connection[];applications:ApplicationGroup[];history:Connection[];selectedId:string|null;firewall:{enabled:boolean|null;enforcement:string;observer:string;error:string|null};lastObserved:number|null;layout:{id:string;y:number}[]}
export type Command = {type:'select';id:string}|{type:'setView';view:View}|{type:'setLocale';locale:Locale}|{type:'setMode';mode:'demo'|'live'}|{type:'setPanel';panel:'left'|'right'|'floating'}|{type:'saveRule';connectionId:string;action:Action;direction:Direction}|{type:'deleteRule';id:string}|{type:'restoreRules';rules:Rule[]};

import type {TextKey} from './i18n';
export const lanColumns = ['address','inspect','change','family','interface','mac','name','nameSource','response','source','observed','comment','label','scope','latency','firstSeen','vendor','services','smb'] as const;
export const lanColumnLabels: Record<string,TextKey> = {change:'lanChange',inspect:'lanInspect',address:'address',family:'protocol',interface:'interfaceName',mac:'lanMac',name:'lanHostname',nameSource:'lanNameSource',response:'lanResponse',source:'lanSource',observed:'lanObserved',comment:'lanComment'};
export type LanEdit = {type:'width';column:string;value:number;expected:number}|{type:'label';id:string;value:string;expected:string}|{type:'archive'}| {type:'selection';ids:string[];expected:string[]}|{type:'comment';id:string;value:string;expected:string}|{type:'columns';values:string[];expected:string[]};
export function validLanColumns(values:string[]) {return values.length<=lanColumns.length&&new Set(values).size===values.length&&values.includes('address')&&values.includes('inspect')&&values.every(x=>(lanColumns as readonly string[]).includes(x));}

export function defaultLanWidth(column:string){return column==='inspect'||column==='family'?80:column==='change'||column==='latency'?120:160;}

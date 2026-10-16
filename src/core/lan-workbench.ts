import type {TextKey} from './i18n';
export const lanColumns = ['change','inspect','address','family','interface','mac','name','nameSource','response','source','observed','comment'] as const;
export const lanColumnLabels: Record<string,TextKey> = {change:'lanChange',inspect:'lanInspect',address:'address',family:'protocol',interface:'interfaceName',mac:'lanMac',name:'lanHostname',nameSource:'lanNameSource',response:'lanResponse',source:'lanSource',observed:'lanObserved',comment:'lanComment'};
export type LanEdit = {type:'selection';ids:string[];expected:string[]}|{type:'comment';id:string;value:string;expected:string}|{type:'columns';values:string[];expected:string[]};
export function validLanColumns(values:string[]) {return values.length<=lanColumns.length&&new Set(values).size===values.length&&values.includes('address')&&values.includes('inspect')&&values.every(x=>(lanColumns as readonly string[]).includes(x));}

import type {SyslogSnapshot,SyslogConfig} from './types';
export const emptySyslog:SyslogSnapshot={operation:0,config:null,stage:'',boundEndpoint:null,counts:{bytesRead:0,frames:0,accepted:0,rejectedSource:0,invalid:0,oversized:0,rateDropped:0,evicted:0,incomplete:0,connections:0},rows:[],selectedId:null,error:null};
export function syslogBusy(stage:string){return ['starting','listening','stopping'].includes(stage);}
export function syslogRows(job:SyslogSnapshot,search:string,severity:string){const query=search.trim().toLowerCase();return job.rows.filter(r=>(severity==='all'||r.priority%8===Number(severity))&&`${r.peer} ${r.text}`.toLowerCase().includes(query)).slice().reverse();}
export function privateIPv4(text:string){const p=text.split('.');if(p.length!==4||p.some(v=>!/^\d{1,3}$/.test(v)||String(Number(v))!==v||Number(v)>255))return false;const [a,b]=p.map(Number);return a===10||a===127||a===172&&b>=16&&b<=31||a===192&&b===168||a===169&&b===254;}
export function validSyslogConfig(c:SyslogConfig){return c.acknowledgeUnverified&&privateIPv4(c.bindAddress)&&privateIPv4(c.sourceAddress)&&(c.bindAddress.startsWith('127.')===c.sourceAddress.startsWith('127.'))&&Number.isInteger(c.port)&&c.port>=1024&&c.port<=65535&&['udp','tcp_newline'].includes(c.transport)&&Number.isInteger(c.durationSeconds)&&c.durationSeconds>=30&&c.durationSeconds<=900;}

export function syslogLevels(job:SyslogSnapshot){const counts=Array<number>(8).fill(0);for(const row of job.rows)counts[row.priority%8]++;return counts;}

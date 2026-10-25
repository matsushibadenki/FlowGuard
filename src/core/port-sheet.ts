import {evidenceRows} from './evidence-sheet';
import type {PortSnapshot} from './types';
export function portRows(job:PortSnapshot,search:string,status:string,descending=false){
 return evidenceRows(job.rows,search,status,row=>[job.target,job.interface,job.sourceAddress,String(row.port),'TCP'].join(' '),row=>row.status,(a,b)=>(descending?-1:1)*(a.port-b.port));
}
export function portScope(ports:number[]){
 if(!ports.length)return '—';
 return ports.length>1&&ports.every((port,i)=>i===0||port===ports[i-1]+1)?`${ports[0]}–${ports.at(-1)}`:ports.join(', ');
}
// Display-side eligibility only. Rust revalidates the fresh interface and on-link prefix.
function ipv6(address:string,interfaceName:string):bigint|null{
 const parts=address.split('%');if(!/^[0-9a-fA-F:.]+$/.test(parts[0])||parts.length>2||(parts.length===2&&parts[1]!==interfaceName))return null;
 try{const canonical=new URL(`http://[${parts[0]}]/`).hostname.slice(1,-1);const halves=canonical.split('::');const left=halves[0]?halves[0].split(':'):[],right=halves[1]?halves[1].split(':'):[];const words=halves.length===2?[...left,...Array(8-left.length-right.length).fill('0'),...right]:left;if(words.length!==8)return null;return words.reduce((value,word)=>(value<<16n)|BigInt(parseInt(word,16)),0n);}catch{return null;}
}
export function eligiblePortDevice(device:Pick<import('./types').LanDevice,'address'|'interface'|'family'>){
 if(device.family==='IPv4'){const raw=device.address.split('.');const parts=raw.map(Number);return /^\d{1,3}(?:\.\d{1,3}){3}$/.test(device.address)&&parts.every((v,i)=>String(v)===raw[i])&&parts.length===4&&parts.every(v=>Number.isInteger(v)&&v>=0&&v<=255)&&(parts[0]===10||(parts[0]===172&&parts[1]>=16&&parts[1]<=31)||(parts[0]===192&&parts[1]===168));}
 if(device.family!=='IPv6')return false;const ip=ipv6(device.address,device.interface);if(ip===null)return false;const first=Number(ip>>112n);return (first&0xffc0)===0xfe80||(first&0xfe00)===0xfc00||(first&0xe000)===0x2000;
}
export function samePortTarget(device:Pick<import('./types').LanDevice,'address'|'interface'|'family'>,job:Pick<PortSnapshot,'target'|'interface'|'family'>){
 if(device.interface!==job.interface||device.family!==job.family)return false;
 return device.family==='IPv6'?ipv6(device.address,device.interface)!==null&&ipv6(device.address,device.interface)===ipv6(job.target,job.interface):device.address===job.target;
}

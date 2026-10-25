import {matchesEvidence} from './evidence-sheet';
import type {NetworkSnapshot} from './types';
export function lanRange(row:NetworkSnapshot['addresses'][number]) {
 const octets=row.address.split('.').map(Number), mask=row.netmask.split('.').map(Number);
 if(row.family!=='IPv4'||octets.length!==4||mask.length!==4||[...octets,...mask].some(n=>!Number.isInteger(n)||n<0||n>255))return null;
 if(!(octets[0]===10||(octets[0]===172&&octets[1]>=16&&octets[1]<=31)||(octets[0]===192&&octets[1]===168)))return null;
 const number=(parts:number[])=>parts.reduce((v,n)=>(v*256+n)>>>0,0);
 const bits=number(mask);let prefix=0;while(prefix<32&&((bits>>>(31-prefix))&1))prefix++;
 if(prefix<24||prefix>30||bits!==(0xffffffff<<(32-prefix))>>>0)return null;
 const start=(number(octets)&bits)>>>0,end=(start|~bits)>>>0;
 if(number(octets)===start||number(octets)===end)return null;
 const text=(n:number)=>[24,16,8,0].map(s=>(n>>>s)&255).join('.');
 return {range:`${text(start+1)}–${text(end-1)} /${prefix}`,total:end-start-2};
}
export function addressCompare(a:string,b:string) {
 const ipv4=(s:string)=>/^\d+\.\d+\.\d+\.\d+$/.test(s)?s.split('.').reduce((v,n)=>v*256+Number(n),0):null;
 const x=ipv4(a),y=ipv4(b);return x!==null&&y!==null?x-y:a.localeCompare(b,undefined,{numeric:true});
}

export function matchesDevice(device:Pick<import('./types').LanDevice,'address'|'mac'|'interface'|'name'>,search:string) {
 return matchesEvidence(`${device.address} ${device.mac??''} ${device.interface} ${device.name.value??''} ${(device.name.values??[]).join(' ')}`,search);
}

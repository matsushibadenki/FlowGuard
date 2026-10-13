import type {PortSnapshot} from './types';
export function portRows(job:PortSnapshot,search:string,status:string,descending=false){
 const needle=search.trim().toLowerCase();
 return job.rows.filter(row=>(status==='all'||row.status===status)&&(!needle||[job.target,job.interface,job.sourceAddress,String(row.port),'TCP'].some(value=>value.toLowerCase().includes(needle)))).sort((a,b)=>(descending?-1:1)*(a.port-b.port));
}
export function portScope(ports:number[]){
 if(!ports.length)return '—';
 return ports.length>1&&ports.every((port,i)=>i===0||port===ports[i-1]+1)?`${ports[0]}–${ports.at(-1)}`:ports.join(', ');
}

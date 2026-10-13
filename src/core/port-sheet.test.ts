import {describe,it,expect} from 'vitest';
import {portRows,portScope} from './port-sheet';
import type {PortSnapshot} from './types';
const job:PortSnapshot={comparison:{status:"first",baselineOperation:null,rows:[]},operation:7,sourceAddress:'192.168.1.20',requestedPorts:[22,80,443],selection:'common',stage:'completed',target:'192.168.1.30',interface:'en0',generation:2,completed:3,total:3,error:null,rows:[{port:443,status:'open',elapsedMs:2,observedAt:1},{port:22,status:'closed',elapsedMs:1,observedAt:1},{port:80,status:'timeout',elapsedMs:600,observedAt:1}]};
describe('TCP sheet',()=>{
 it('filters by result status and port without mutating captured rows',()=>{expect(portRows(job,'','open').map(r=>r.port)).toEqual([443]);expect(portRows(job,'80','all').map(r=>r.status)).toEqual(['timeout']);expect(portRows(job,'TCP','all',true).map(r=>r.port)).toEqual([443,80,22]);expect(job.rows.map(r=>r.port)).toEqual([443,22,80]);});
 it('finds source and interface and reports no matches separately',()=>{expect(portRows(job,' EN0 ','all')).toHaveLength(3);expect(portRows(job,'192.168.1.20','all')).toHaveLength(3);expect(portRows(job,'missing','all')).toHaveLength(0);});
 it('shows exact requested ports and compresses only consecutive ranges',()=>{expect(portScope([22,80,443])).toBe('22, 80, 443');expect(portScope([10,11,12])).toBe('10–12');expect(portScope([443])).toBe('443');expect(portScope([])).toBe('—');});
});

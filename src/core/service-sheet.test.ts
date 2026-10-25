import {describe,it,expect} from 'vitest';
import {serviceComparisonWords} from './service-comparison';
import {serviceRows,serviceAvailability,staleServices} from './service-sheet';
import type {Snapshot,ServiceSnapshot} from './types';
const job:ServiceSnapshot={operation:2,portOperation:1,generation:3,target:'192.168.1.30',sourceAddress:'192.168.1.20',interface:'en0',family:'IPv4',mode:'auto',stage:'completed',total:2,completed:2,error:null,selectedPort:null,rows:[{port:443,candidate:'HTTPS',service:null,status:'tls_error',method:'https',product:null,response:null,tls:null,observedAt:1},{port:22,candidate:'SSH',service:'SSH',status:'confirmed',method:'ssh',product:'ExampleSSH/1',response:'SSH-2.0-ExampleSSH/1',tls:null,observedAt:1}]};
const snapshot={services:job,ports:{operation:1,generation:3,stage:'completed',error:null,rows:[{port:22,status:'open'}]},inventory:{generation:3},discovery:{stage:'completed'}} as Snapshot;
describe('service evidence sheet',()=>{
 it('filters reported metadata while preserving unknown and failed observations',()=>{expect(serviceRows(job,'','all').map(r=>r.port)).toEqual([22,443]);expect(serviceRows(job,'ExampleSSH','all')).toHaveLength(1);expect(serviceRows(job,'','tls_error')[0].service).toBeNull();expect(serviceRows(job,'','all',true).map(r=>r.port)).toEqual([443,22]);expect(job.rows[0].port).toBe(443);});
 it('requires current complete open-port results and reports stale evidence',()=>{expect(serviceAvailability(snapshot)).toBe(true);expect(serviceAvailability({...snapshot,ports:{...snapshot.ports,stage:'running'}})).toBe(false);expect(serviceAvailability({...snapshot,ports:{...snapshot.ports,rows:[]}})).toBe(false);expect(serviceAvailability({...snapshot,services:{...job,stage:'cancelling'}})).toBe(false);expect(staleServices(snapshot)).toBe(false);expect(staleServices({...snapshot,ports:{...snapshot.ports,operation:2}})).toBe(true);expect(staleServices({...snapshot,inventory:{...snapshot.inventory,generation:4}})).toBe(true);});
});

describe('service comparison filtering',()=>{
 const compared:ServiceSnapshot={...job,comparison:{status:'uncertain',baselineOperation:1,baselinePortOperation:1,rows:[{port:22,kind:'unchanged',fields:[],previous:job.rows[1]},{port:443,kind:'uncertain',fields:['status'],previous:{...job.rows[0],status:'confirmed',service:'HTTPS'}}]}};
 it('keeps uncertain rows visible and sorts only the projected results',()=>{expect(serviceRows(compared,'','all',false,true).map(r=>r.port)).toEqual([443]);expect(serviceRows(compared,'','tls_error',true,true).map(r=>r.port)).toEqual([443]);expect(serviceRows(compared,'ExampleSSH','all',false,true)).toEqual([]);expect(serviceRows(compared,'ExampleSSH','all')).toHaveLength(1);expect(compared.rows).toHaveLength(2);});
 it('does not hide the results when comparison becomes unavailable',()=>{for(const status of ['first','scope_changed','running','invalidated','incomplete'])expect(serviceRows({...compared,comparison:{...compared.comparison!,status}},'','all',false,true)).toHaveLength(2);expect(serviceRows({...compared,stage:'cancelled'},'','all',false,true)).toHaveLength(2);});
 it('shows an empty changes filter when all observed evidence is unchanged',()=>{const unchanged={...compared,comparison:{...compared.comparison!,status:'ready',rows:compared.comparison!.rows.map(r=>({...r,kind:'unchanged' as const}))}};expect(serviceRows(unchanged,'','all',false,true)).toHaveLength(0);});
});

it('has complete comparison labels in three languages',()=>{for(const locale of ['ja','en','zh-CN'] as const)expect(Object.keys(serviceComparisonWords[locale]).sort()).toEqual(Object.keys(serviceComparisonWords.en).sort());});

import {describe,it,expect} from 'vitest';
import {installAllowed,updateBusy,updateProgress} from './updates';
import {emptySyslog} from './syslog';
import type {Snapshot,UpdateSnapshot} from './types';
const updates:UpdateSnapshot={stage:'ready',operation:1,currentVersion:'0.1.0',version:'0.2.0',notes:null,checkedAt:1,downloaded:25,total:100,error:null,development:false};
const state={updates,extension:{busy:false,stage:'disabled'},ports:{stage:'completed'},services:{stage:'completed'},discovery:{stage:'completed'}} as Snapshot;
describe('software update controls',()=>{
 it('blocks installation during OS firewall authentication or changes',()=>expect(installAllowed({...state,firewallControl:{operation:1,busy:true,service:'enabled',enabled:null,error:null}})).toBe(false));
 it('only offers installation for verified release builds with no active jobs',()=>{expect(installAllowed(state)).toBe(true);expect(installAllowed({...state,updates:{...updates,development:true}})).toBe(false);expect(installAllowed({...state,updates:{...updates,stage:'available'}})).toBe(false);for(const key of ['ports','services','discovery'] as const)expect(installAllowed({...state,[key]:{...state[key],stage:'cancelling'}})).toBe(false);expect(installAllowed({...state,extension:{...state.extension,busy:true}})).toBe(false);expect(installAllowed({...state,extension:{...state.extension,stage:'restart_required'}})).toBe(false);});
 it('blocks installation while a router collection is active',()=>expect(installAllowed({...state,security:{...state.security,routerSources:[{id:'home',endpoint:'192.168.1.1:443',location:'unknown',stage:'collecting',authentication:'unknown',sampledAt:null,error:null}]}})).toBe(false));
 it('blocks update installation during syslog startup, listening and shutdown',()=>{for(const stage of ['starting','listening','stopping'])expect(installAllowed({...state,syslog:{...emptySyslog,stage}})).toBe(false);});
 it('shows bounded and indeterminate progress and treats installing as busy',()=>{expect(updateProgress(updates)).toBe(25);expect(updateProgress({...updates,downloaded:200})).toBe(100);expect(updateProgress({...updates,total:null})).toBeNull();expect(updateBusy({...updates,stage:'installing'})).toBe(true);expect(updateBusy(updates)).toBe(false);});
});

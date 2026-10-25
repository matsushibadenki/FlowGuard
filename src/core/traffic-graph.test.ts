// パス: src/core/traffic-graph.test.ts
import {describe,it,expect} from 'vitest';
import {destinationReceipt,pageWindow,trafficGraphWords} from './traffic-graph';
import {buildAppRows} from './mapModel';
import {groupApplications} from './applications';
import {defaultExtension} from './enforcement';
import type {Connection,Rule,Snapshot} from './types';
const identity={authority:'developer' as const,teamId:'ABCDE12345',identifier:'com.test.graph'};
const connection:Connection={id:'c1',identity,app:'Example',appKey:'/same-path',pid:1,local:'127.0.0.1',destination:'1.1.1.1',port:443,protocol:'TCP',direction:'outbound',observedAt:1};
const rule:Rule={id:'r',identity,app:'Example',appKey:'signed:v1:developer:ABCDE12345:com.test.graph',source:'live',action:'block',destination:'1.1.1.1',port:443,direction:'both',updatedAt:1};
function make(){return {connections:[connection],document:{preferences:{mode:'live'},rules:[rule]},extension:{...defaultExtension,signed:true,instanceId:'provider',providerRevision:1,stage:'applied',appliedRuleIds:['r']},firewall:{error:null}} as unknown as Snapshot;}
function project(s:Snapshot){return buildAppRows({groups:groupApplications(s.connections),connections:s.connections,rules:s.document.rules,extension:s.extension,mode:s.document.preferences.mode,hideLoopback:false,sort:'name',locale:'en'}).rows[0];}
describe('traffic graph evidence',()=>{
 it('keeps saved intent on receipt loss without claiming blocking',()=>{const s=make();expect(destinationReceipt(project(s).destinations[0],s)).toBe('confirmed');for(const stage of ['disconnected','stale','applying','initial_sync','disabled']){s.extension.stage=stage;const d=project(s).destinations[0];expect(d.intent).toBe('block');expect(destinationReceipt(d,s)).toBe('unconfirmed');}s.extension.stage='applied';s.firewall.error='lost';expect(destinationReceipt(project(s).destinations[0],s)).toBe('unconfirmed');s.document.preferences.mode='demo';expect(destinationReceipt(project(s).destinations[0],s)).toBe('demo');});
 it('does not merge same-path sockets with different signature matches',()=>{const s=make();s.connections=[connection,{...connection,id:'unsigned',identity:null},{...connection,id:'other',identity:{...identity,identifier:'com.other'}}];const row=project(s);expect(row.connectionCount).toBe(3);expect(row.destinations).toHaveLength(2);expect(row.destinations.find(d=>d.intent==='block')?.connectionIds).toEqual(['c1']);expect(row.destinations.find(d=>d.intent==='none')?.connectionIds).toEqual(['unsigned','other']);});
 it('shows system protection and unsupported UDP without a false receipt',()=>{const s=make(),apple={authority:'apple' as const,teamId:'',identifier:'com.apple.mDNSResponder'};s.connections=[{...connection,identity:apple,port:53}];s.document.rules=[{...rule,identity:apple,appKey:'signed:v1:apple::com.apple.mDNSResponder',port:53}];const d=project(s).destinations[0];expect(destinationReceipt(d,s)).toBe('protected');s.connections=[{...connection,protocol:'UDP'}];s.document.rules=[rule];expect(destinationReceipt(project(s).destinations[0],s)).toBe('none');});
 it('provides all apps and endpoints across bounded pages and clamps after filtering',()=>{const rows=Array.from({length:172},(_,i)=>i);const shown=[];for(let p=0;p<43;p++)shown.push(...pageWindow(rows,p,4).rows);expect(shown).toEqual(rows);expect(pageWindow(rows.slice(0,3),42,4).current).toBe(0);expect(pageWindow([],7,4).rows).toEqual([]);});
 it('has matching labels in three languages',()=>{for(const w of Object.values(trafficGraphWords))expect(Object.keys(w).sort()).toEqual(Object.keys(trafficGraphWords.en).sort());});
});

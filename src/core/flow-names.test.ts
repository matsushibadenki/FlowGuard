import {describe,it,expect} from 'vitest';
import {flowNamesFor,flowNameText,flowNameWords} from './flow-names';
import type {Snapshot} from './types';
const now=Math.floor(Date.now()/1000);
const snapshot={document:{preferences:{mode:'live'}},flowNames:{enabled:true,searchEnabled:false,busy:false,operation:1,error:null,rows:[{connectionId:'one',values:[{value:'one.example',source:'network_extension_remote_hostname',trust:'unverified_tuple_time_candidate',observedAt:now},{value:'two.example',source:'network_extension_remote_hostname',trust:'unverified_tuple_time_candidate',observedAt:now}]}]}} as Snapshot;
describe('passive OS flow name evidence',()=>{
 it('keeps multiple names separate and searches them only after explicit opt-in',()=>{expect(flowNamesFor(snapshot,['one'])).toHaveLength(2);expect(flowNameText(snapshot,'one')).toBe('');expect(flowNameText({...snapshot,flowNames:{...snapshot.flowNames!,searchEnabled:true}},'one')).toBe('one.example two.example');expect(flowNamesFor(snapshot,['another'])).toEqual([]);});
 it('hides revoked, demo, stale, future and other-source evidence',()=>{expect(flowNamesFor({...snapshot,flowNames:{...snapshot.flowNames!,enabled:false}},['one'])).toEqual([]);expect(flowNamesFor({...snapshot,document:{...snapshot.document,preferences:{...snapshot.document.preferences,mode:'demo'}}},['one'])).toEqual([]);const copy=structuredClone(snapshot);copy.flowNames!.rows[0].values[0].observedAt=now-16;copy.flowNames!.rows[0].values[1].observedAt=now+10;expect(flowNamesFor(copy,['one'])).toEqual([]);});
 it('provides matched labels in three languages',()=>{for(const w of Object.values(flowNameWords))expect(Object.keys(w).sort()).toEqual(Object.keys(flowNameWords.en).sort());});
});

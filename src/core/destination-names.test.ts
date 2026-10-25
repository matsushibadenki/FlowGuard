import {describe,it,expect} from 'vitest';
import {destinationName,destinationNameWords,emptyDestinationNames,nameLookupEligible} from './destination-names';
import {installAllowed} from './updates';
import type {Snapshot,DestinationNameRow} from './types';
const row:DestinationNameRow={address:'192.0.2.1',value:'host.example',source:'system_resolver',observedAt:1,status:'resolved',stale:false,trust:'unverified_name'};
const state={document:{preferences:{mode:'live'}},destinationNames:{...emptyDestinationNames,enabled:true,rows:[row]}} as Snapshot;
describe('optional destination names',()=>{
 it('only displays evidence for the exact IP in live mode with consent',()=>{
  expect(destinationName(state,'192.0.2.1')).toEqual(row);expect(destinationName(state,'192.0.2.2')).toBeUndefined();expect(destinationName({...state,destinationNames:{...state.destinationNames!,enabled:false}},row.address)).toBeUndefined();expect(destinationName({...state,document:{...state.document,preferences:{...state.document.preferences,mode:'demo'}}},row.address)).toBeUndefined();
 });
 it('does not offer loopback, scoped, multicast or non-IP lookups',()=>{for(const a of ['192.0.2.1','2001:db8::1','fd12::1'])expect(nameLookupEligible(a)).toBe(true);for(const a of ['127.0.0.1','::ffff:127.0.0.1','::1','::','224.0.0.1','fe80::1','fe80::1%en0','host.example'])expect(nameLookupEligible(a)).toBe(false);});
 it('has localized source, time, unverified, stale and outcome controls',()=>{const keys=Object.keys(destinationNameWords.en).sort();for(const w of Object.values(destinationNameWords)){expect(Object.keys(w).sort()).toEqual(keys);expect(Object.values(w).every(Boolean)).toBe(true);}});
 it('blocks update installation until a cancelled worker actually finishes',()=>{const s={updates:{stage:'ready',development:false},extension:{busy:false,stage:'disabled'},discovery:{stage:'completed'},ports:{stage:'completed'},services:{stage:'completed'},destinationNames:{...emptyDestinationNames,busy:true,cancelling:true}} as Snapshot;expect(installAllowed(s)).toBe(false);expect(installAllowed({...s,destinationNames:emptyDestinationNames})).toBe(true);});
});

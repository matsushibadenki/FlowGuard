import {describe,it,expect} from 'vitest';
import {emptyFirewallControl,firewallWriteAllowed,firewallWords} from './firewall-control';
describe('macOS firewall controls',()=>{
 it('requires desktop, confirmation, registered helper and verified known state',()=>{
  const valid={...emptyFirewallControl,service:'enabled',enabled:true};
  expect(firewallWriteAllowed(valid,true,true)).toBe(true);
  for(const s of [emptyFirewallControl,{...valid,busy:true},{...valid,enabled:null},{...valid,service:'requires_approval'},{...valid,error:'result_unknown'}])expect(firewallWriteAllowed(s,true,true)).toBe(false);
  expect(firewallWriteAllowed(valid,false,true)).toBe(false);
  expect(firewallWriteAllowed(valid,true,false)).toBe(false);
 });
 it('provides all controls and uncertainty messages in three languages',()=>{
  const keys=Object.keys(firewallWords.en).sort();
  for(const words of Object.values(firewallWords)){expect(Object.keys(words).sort()).toEqual(keys);expect(Object.values(words).every(Boolean)).toBe(true);}
 });
});

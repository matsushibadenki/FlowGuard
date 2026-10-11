import {describe,it,expect} from 'vitest';
import {lanRange,addressCompare,matchesDevice} from './lan';
const row=(address:string,netmask='255.255.255.0')=>({interface:'en0',address,netmask,family:'IPv4',up:true});
describe('LAN scope and sheet',()=>{
 it('shows the same bounded private subnet and excludes this Mac',()=>{
  expect(lanRange(row('192.168.1.20'))).toEqual({range:'192.168.1.1–192.168.1.254 /24',total:253});
  expect(lanRange(row('10.0.0.1','255.255.255.252'))?.total).toBe(1);
 });
 it('disables public, oversized, invalid-mask and IPv6 active scopes',()=>{
  for(const r of [row('8.8.8.8'),row('10.0.0.2','255.255.0.0'),row('10.0.0.2','255.255.255.1'),{...row('fe80::1%en0'),family:'IPv6'}])expect(lanRange(r)).toBeNull();
 });
 it('orders IPv4 numerically and retains scoped IPv6 strings',()=>{
  expect(['192.168.1.100','192.168.1.9','192.168.1.2'].sort(addressCompare)).toEqual(['192.168.1.2','192.168.1.9','192.168.1.100']);
  expect(addressCompare('fe80::1%en0','fe80::2%en0')).toBeLessThan(0);
 });
 it('searches collected names without manufacturing an address identity',()=>{
  const device={address:'192.168.1.2',mac:null,interface:'en0',name:{value:'Printer.local',source:'mdns',status:'resolved',observedAt:1}};
  expect(matchesDevice(device,'PRINTER')).toBe(true);expect(matchesDevice({...device,name:{...device.name,value:null}},'printer')).toBe(false);
 });
});

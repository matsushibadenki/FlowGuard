import {describe,it,expect} from 'vitest';
import {systemPreset,systemProtected} from './systemProtection';
import {ruleApplied,defaultExtension} from './enforcement';
import {validRules} from './rules';
import type {Rule} from './types';
const rule:Rule={id:'r',app:'DNS',appKey:'signed:v1:apple::com.apple.mDNSResponder',identity:{authority:'apple',teamId:'',identifier:'com.apple.mDNSResponder'},destination:'1.1.1.1',port:53,direction:'both',action:'block',source:'live',updatedAt:1};
describe('system protection',()=>{
 it('requires Apple subject and service, never name/path/port alone',()=>{
  expect(systemPreset(rule,'TCP')).toBe('systemDNS');
  expect(systemPreset({...rule,identity:null},'TCP')).toBeNull();
  expect(systemPreset({...rule,identity:{authority:'developer',teamId:'ABCDE12345',identifier:'com.apple.mDNSResponder'}},'TCP')).toBeNull();
  expect(systemPreset({...rule,direction:'inbound'},'TCP')).toBeNull();
  expect(systemPreset({...rule,port:443},'TCP')).toBeNull();
 });
 it('requires an explicit supported transport before claiming any preset',()=>{
  for(const protocol of ['UDP','unknown','','tcp','ICMP'])expect(systemPreset(rule,protocol)).toBeNull();
  expect(systemPreset({...rule,port:5353},'UDP')).toBeNull();
  for(const port of [22,80,443,3389,5900])expect(systemPreset({...rule,port},'TCP')).toBeNull();
 });
 it('defaults legacy rules to protected and isolates demo/explicit exceptions',()=>{
  expect(systemProtected(rule)).toBe(true);
  expect(ruleApplied(rule,{...defaultExtension,stage:'applied',signed:true,appliedRuleIds:[rule.id]})).toBe(false);
  expect(systemProtected({...rule,systemException:true})).toBe(false);
  expect(systemProtected({...rule,source:'demo'})).toBe(false);
  expect(validRules([{...rule,systemException:'true'}])).toBe(false);
  expect(validRules([rule,{...rule,id:'other',systemException:true}])).toBe(true);
 });
});

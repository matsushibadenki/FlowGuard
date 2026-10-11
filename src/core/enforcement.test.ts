import {describe,it,expect} from 'vitest';
import {defaultExtension,ruleApplied,policyNotice,extensionLabel} from './enforcement';
import {dictionaries} from './i18n';
import type {Rule} from './types';
const rule:Rule={id:'r',app:'Test',appKey:'/test',destination:'example.com',port:443,direction:'both',action:'block',source:'live',updatedAt:1};
describe('provider acknowledgement UI contract',()=>{
 it('requires current applied state and a live rule receipt',()=>{
  const applied={...defaultExtension,stage:'applied',appliedRuleIds:['r']};
  expect(ruleApplied(rule,applied)).toBe(true);
  expect(ruleApplied({...rule,source:'demo'},applied)).toBe(false);
  for(const stage of ['ready','configuration_saved','stale','failed','disconnected','applying'])expect(ruleApplied(rule,{...applied,stage})).toBe(false);
  expect(ruleApplied(rule,{...applied,busy:true})).toBe(false);
 });
 it('explains stale and uncertain policies instead of claiming observation only',()=>{
  expect(policyNotice({...defaultExtension,stage:'stale'},'live')).toBe('stalePolicyHelp');
  expect(policyNotice({...defaultExtension,stage:'disconnected'},'live')).toBe('uncertainPolicyHelp');
  expect(policyNotice({...defaultExtension,stage:'applied'},'demo')).toBe('demoNeverApplied');
 });
 it('has translated labels for every lifecycle stage',()=>{
  for(const stage of ['not_packaged','runtime_unavailable','signature_required','inactive','activating','approval_required','restart_required','configuration_saved','ready','applying','applied','stale','disconnected','failed','configuration_conflict']){
   const key=extensionLabel(stage);expect(key).not.toBe('unknown');
   for(const dictionary of Object.values(dictionaries))expect(dictionary[key].length).toBeGreaterThan(0);
  }
 });
});

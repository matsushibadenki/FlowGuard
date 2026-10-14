import {describe,it,expect} from 'vitest';
const signing = {authority:'developer' as const,teamId:'ABCDE12345',identifier:'com.test.app'};
const subject = 'signed:v1:developer:ABCDE12345:com.test.app';
import {protectionLabel,defaultExtension,ruleApplied,policyNotice,extensionLabel} from './enforcement';
import {dictionaries} from './i18n';
import type {Rule} from './types';
const rule:Rule={identity:signing,id:'r',app:'Test',appKey:subject,destination:'example.com',port:443,direction:'both',action:'block',source:'live',updatedAt:1};
describe('provider acknowledgement UI contract',()=>{
 it('requires current applied state and a live rule receipt',()=>{
  const applied={...defaultExtension,stage:'applied',appliedRuleIds:['r']};
  expect(ruleApplied(rule,applied)).toBe(true);
  expect(ruleApplied({...rule,source:'demo'},applied)).toBe(false);
  for(const stage of ['initial_sync','disabled','disabling','removed','removing','ready','configuration_saved','stale','failed','disconnected','applying'])expect(ruleApplied(rule,{...applied,stage})).toBe(false);
  expect(ruleApplied(rule,{...applied,busy:true})).toBe(false);
 });
 it('explains stale and uncertain policies instead of claiming observation only',()=>{
  expect(policyNotice({...defaultExtension,stage:'stale'},'live')).toBe('stalePolicyHelp');
  expect(policyNotice({...defaultExtension,stage:'disconnected'},'live')).toBe('uncertainPolicyHelp');
  expect(policyNotice({...defaultExtension,stage:'applied'},'demo')).toBe('demoNeverApplied');
 });
 it('has translated labels for every lifecycle stage',()=>{
  for(const stage of ['initial_sync','disabled','disabling','removed','removing','invalid_initial_policy','not_packaged','runtime_unavailable','signature_required','inactive','activating','approval_required','restart_required','configuration_saved','ready','applying','applied','stale','disconnected','failed','configuration_conflict']){
   const key=extensionLabel(stage);expect(key).not.toBe('unknown');
   for(const dictionary of Object.values(dictionaries))expect(dictionary[key].length).toBeGreaterThan(0);
  }
 });
});

describe('top-level protection status',()=>{
 it('never carries a confirmation through a lost, busy or demo state',()=>{
  const receipt={...defaultExtension,signed:true,stage:'applied',instanceId:'boot',providerRevision:1};
  expect(protectionLabel(receipt,'live',true)).toBe('protectionConfirmed');
  for(const stage of ['stale','failed','disconnected','applying'])expect(protectionLabel({...receipt,stage},'live',true)).toBe('protectionUncertain');
  expect(protectionLabel({...receipt,busy:true},'live',true)).toBe('protectionUncertain');
  expect(protectionLabel(receipt,'demo',true)).toBe('protectionDemo');
  expect(protectionLabel({...receipt,instanceId:null},'live',true)).toBe('protectionSaved');
  expect(protectionLabel(defaultExtension,'live',false)).toBe('protectionObserved');
 });
});

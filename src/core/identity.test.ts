// パス: src/core/identity.test.ts
import {describe, expect, it} from 'vitest';
import {canSaveRule, sameIdentity, identityKey} from './identity';
import {ruleFor, validRules} from './rules';
import {ruleApplied, defaultExtension} from './enforcement';
import type {Connection, Rule} from './types';
const identity = {authority: 'developer' as const, teamId: 'ABCDE12345', identifier: 'com.test.app'};
const connection: Connection = {id:'c',app:'Test',appKey:'/Applications/Test',pid:1,local:'::1',destination:'1.2.3.4',port:443,protocol:'TCP',direction:'unknown',observedAt:1,identity};
const legacy: Rule = {id:'old',app:'Test',appKey:connection.appKey,destination:connection.destination,port:443,direction:'both',action:'allow',source:'live',updatedAt:1};
const signed: Rule = {...legacy, id:'new', appKey:identityKey(identity),identity};
describe('signed subjects and safe legacy imports', () => {
  it('retains old backups without making their path rules eligible', () => {
    expect(validRules([legacy])).toBe(true);
    expect(ruleFor([legacy],connection,'live')).toBeUndefined();
    expect(ruleApplied(legacy,{...defaultExtension,stage:'applied',appliedRuleIds:['old']})).toBe(false);
  });
  it('matches moves and updates but rejects another publisher or identifier at the same path', () => {
    expect(ruleFor([signed],{...connection,appKey:'/Moved/Test'},'live')).toBe(signed);
    for(const changed of [{...identity,teamId:'ZZZZZ12345'},{...identity,identifier:'com.impostor'}]) {
      expect(ruleFor([signed],{...connection,identity:changed},'live')).toBeUndefined();
    }
  });
  it('never saves or matches a TCP rule against UDP or unknown transport',()=>{for(const protocol of ['UDP','unknown',''])for(const mode of ['live','demo'] as const){expect(canSaveRule({...connection,protocol},mode)).toBe(false);expect(ruleFor([signed],{...connection,protocol},mode)).toBeUndefined();}});
  it('never matches or saves an unverified live process', () => {
    for(const identity of [null,undefined]) {
      expect(canSaveRule({...connection,identity},'live')).toBe(false);
      expect(ruleFor([signed],{...connection,identity},'live')).toBeUndefined();
    }
    expect(canSaveRule({...connection,identity:null},'demo')).toBe(true);
  });
  it('rejects invalid subjects, inconsistent keys and false demo signature claims', () => {
    expect(sameIdentity(identity,{...identity,identifier:'x" or true'})).toBe(false);
    expect(validRules([{...signed,appKey:legacy.appKey}])).toBe(false);
    expect(validRules([{...signed,source:'demo'}])).toBe(false);
    expect(validRules([{...signed,identity:{...identity,teamId:''}}])).toBe(false);
  });
});

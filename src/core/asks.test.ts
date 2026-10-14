// パス: src/core/asks.test.ts
import {describe, expect, it} from 'vitest';
import {validRules} from './rules';
import type {Rule} from './types';
const rule: Rule = {id:'demo',app:'Test',appKey:'/test',destination:'1.2.3.4',port:443,direction:'both',action:'ask',source:'demo',updatedAt:1};
describe('ask backup policy options',()=>{
 it('keeps pre-option backups valid, accepts both decisions and rejects ambiguous fallback values',()=>{
  expect(validRules([rule])).toBe(true);
  for(const timeoutAction of ['allow','block'])expect(validRules([{...rule,timeoutAction}])).toBe(true);
  for(const timeoutAction of ['ask','execute','',null])expect(validRules([{...rule,timeoutAction}])).toBe(false);
 });
});

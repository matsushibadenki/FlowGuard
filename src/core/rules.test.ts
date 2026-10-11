import {describe,it,expect} from 'vitest';
import {ruleFor,validRules,endpoint} from './rules';
import type {Connection,Rule} from './types';
const c:Connection={id:'c',app:'Mail',appKey:'/Mail',pid:1,local:'::1',destination:'2001:db8::1',port:993,protocol:'TCP',direction:'unknown',observedAt:1};
const r:Rule={id:'r',appKey:'/Mail',app:'Mail',destination:c.destination,port:993,direction:'both',action:'block',updatedAt:1,source:'live'};
describe('rule contracts',()=>{
 it('does not conflate intent with a different source or port',()=>{expect(ruleFor([r],c,'demo')).toBeUndefined();expect(ruleFor([{...r,port:443}],c,'live')).toBeUndefined();expect(ruleFor([r],c,'live')).toBe(r);});
 it('does not infer an outbound rule for unknown direction',()=>expect(ruleFor([{...r,direction:'outbound'}],c,'live')).toBeUndefined());
 it('rejects invalid and duplicate imported rules',()=>{expect(validRules([r])).toBe(true);expect(validRules([r,r])).toBe(false);expect(validRules([{...r,port:70000}])).toBe(false);expect(validRules([{...r,action:'execute'}])).toBe(false);});
 it('specific direction overrides a both-directions fallback',()=>{const outgoing={...c,direction:'outbound'};const specific={...r,id:'s',direction:'outbound' as const,action:'allow' as const,updatedAt:2};expect(ruleFor([r,specific],outgoing,'live')).toBe(specific);});
 it('formats IPv6 endpoints unambiguously',()=>expect(endpoint(c)).toBe('[2001:db8::1]:993'));
});

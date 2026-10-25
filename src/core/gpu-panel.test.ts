import fixture from './fixtures/gpu-filters.json';
import {ruleFor} from './rules';
import {classifyAddress} from './destinations';
import type {Connection,Rule} from './types';
import {describe,it,expect} from 'vitest';
import {clipBounds,gpuWords} from './gpu-panel';
describe('native GPU panel viewport',()=>{
 const rect={x:20,y:100,width:800,height:320};
 it('clips a scrolled drawable without changing its original geometry',()=>{
  const r={...rect,y:-100};const bounds=clipBounds(r,1000,700);expect(bounds?.rect).toEqual(r);expect(bounds?.clip).toEqual({x:20,y:0,width:800,height:220});
 });
 it('respects scroll containers and the protection header',()=>{
  expect(clipBounds(rect,1000,700,[{x:50,y:160,width:600,height:500}])?.clip).toEqual({x:50,y:160,width:600,height:260});
  expect(clipBounds(rect,1000,700,[{x:0,y:500,width:1000,height:100}])).toBeNull();
 });
 it('hides zero, offscreen, oversized and invalid viewports',()=>{
  for(const r of [{...rect,width:0},{...rect,x:NaN},{...rect,width:4097},{...rect,y:900}])expect(clipBounds(r,1000,700)).toBeNull();
 });
 it('keeps all localized controls and observation caveats',()=>{const keys=Object.keys(gpuWords.en).sort();for(const words of Object.values(gpuWords)){expect(Object.keys(words).sort()).toEqual(keys);expect(Object.values(words).every(Boolean)).toBe(true);}});
});

describe('GPU/normal map shared projection contract',()=>{
 it('matches the same cases in Rust and the real normal map helpers',()=>{
  for(const {filter:f,expected} of fixture.cases){
   const ids=(fixture.connections as Connection[]).filter(c=>`${c.app} ${c.destination} ${c.appKey}`.toLowerCase().includes(f.search.toLowerCase())&&(f.intent==='all'||(ruleFor(fixture.rules as Rule[],c,f.source)?.action??'none')===f.intent)&&(!f.hideLoopback||classifyAddress(c.destination)!=='loopback')).map(c=>c.id);
   expect(ids,JSON.stringify(f)).toEqual(expected);
  }
 });
});

import {describe,it,expect} from 'vitest';
import {compactPortColumns,defaultPortLayout,portTableWidth,validPortLayout,portLayoutWords} from './port-layout';
describe('port sheet presentation',()=>{
 it('keeps essential result and selection columns while allowing a compact view',()=>{
  expect(validPortLayout(defaultPortLayout)).toBe(true);
  expect(validPortLayout({...defaultPortLayout,columns:compactPortColumns})).toBe(true);
  for(const columns of [[],['port','inspect'],['status','port','inspect'],['port','status','inspect','inspect'],['port','status','inspect','other']])expect(validPortLayout({...defaultPortLayout,columns} as typeof defaultPortLayout)).toBe(false);
 });
 it('bounds widths and derives the actual visible table width',()=>{
  const compact={...defaultPortLayout,columns:compactPortColumns};expect(portTableWidth(compact)).toBe(470);
  for(const width of [0,79,401,Infinity,NaN,120.5])expect(validPortLayout({...compact,widths:{...compact.widths,port:width}})).toBe(false);
  expect(validPortLayout({...compact,widths:{...compact.widths,port:400}})).toBe(true);
  expect(validPortLayout({...compact,widths:{...compact.widths,unknown:80}} as typeof defaultPortLayout)).toBe(false);
 });
 it('supplies all layout controls and failure feedback in three languages',()=>{
  const keys=Object.keys(portLayoutWords.ja);for(const locale of ['en','zh-CN'] as const){expect(Object.keys(portLayoutWords[locale])).toEqual(keys);expect(Object.values(portLayoutWords[locale]).every(Boolean)).toBe(true);}
 });
});

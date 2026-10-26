import {describe,it,expect} from 'vitest';
import {validServiceNote,serviceNoteWords} from './service-notes';
import {serviceRows} from './service-sheet';
import type {ServiceSnapshot} from './types';
describe('service user notes',()=>{
 it('counts Unicode code points and accepts only supported whitespace',()=>{
  expect(validServiceNote('界'.repeat(80),'😀'.repeat(500))).toBe(true);expect(validServiceNote('界'.repeat(81),'')).toBe(false);expect(validServiceNote('','😀'.repeat(501))).toBe(false);
  expect(validServiceNote('display','line1\nline2\ttab')).toBe(true);for(const control of ['\0','\u001b','\u0085','\r'])expect(validServiceNote('ok',control)).toBe(false);expect(validServiceNote('a\nb','')).toBe(false);
 });
 it('searches saved names and comments without editing observed names or TLS fields',()=>{
  const job={target:'192.168.1.1',interface:'en0',rows:[{port:443,status:'confirmed',candidate:'HTTPS',service:'HTTPS',product:null,response:null}],notes:{443:{label:'管理コンソール',comment:'更新後に再確認',revision:1,updatedAt:1}}} as unknown as ServiceSnapshot;
  expect(serviceRows(job,'管理コンソール','all')).toHaveLength(1);expect(serviceRows(job,'再確認','all')).toHaveLength(1);expect(serviceRows(job,'管理コンソール','timeout')).toHaveLength(0);expect(job.target).toBe('192.168.1.1');expect(job.rows[0].candidate).toBe('HTTPS');
 });
 it('provides the same editing, conflict and evidence boundaries in three languages',()=>{for(const locale of ['en','zh-CN'] as const){expect(Object.keys(serviceNoteWords[locale])).toEqual(Object.keys(serviceNoteWords.ja));expect(Object.values(serviceNoteWords[locale]).every(Boolean)).toBe(true);}});
});

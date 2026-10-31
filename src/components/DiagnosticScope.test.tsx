import {describe,expect,it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {DiagnosticScope} from './DiagnosticScope';
import type {DiagnosticFinding,DiagnosticReport} from '../core/types';

const report:DiagnosticReport={id:1,serviceOperation:2,generation:1,target:'192.0.2.1',interface:'en0',sourceAddress:'192.0.2.2',family:'IPv4',createdAt:1,declaredScope:{revision:1,purpose:'Web',environment:'production',owner:'Alex',exposure:'LAN only'},findings:[{id:'a',port:443,observedAt:1,category:'service',certainty:'observed'},{id:'b',port:22,observedAt:1,category:'service',certainty:'unverified'},{id:'c',category:'tls',certainty:'observed'}] as DiagnosticFinding[]};
describe('exposure review',()=>{
 it('separates the saved declaration, source and observed service counts',()=>{
  const html=renderToStaticMarkup(<DiagnosticScope report={report} locale="ja" native selectedFinding="a" onSelectFinding={()=>{}}/>);
  expect(html).toContain('LAN only');
  expect(html).toContain('192.0.2.2');
  expect(html).toContain('応答を確認 1');
  expect(html).toContain('応答未確認 1');
  expect(html).toContain('aria-pressed="true"');
  expect(html).toContain('aria-label="診断詳細を開く: 443');
  expect(html).toContain('aria-pressed="false"');
  expect(html).toContain('LAN内からの応答はインターネット公開を証明せず');
 });
 it('keeps missing declarations and sources unknown',()=>{
  const html=renderToStaticMarkup(<DiagnosticScope report={{...report,declaredScope:undefined,sourceAddress:undefined,findings:[]}} locale="en" native onSelectFinding={()=>{}}/>);
  expect(html).toContain('Not recorded');
  expect(html).toContain('Unknown');
  expect(html).toContain('Responses observed 0');
 });
});

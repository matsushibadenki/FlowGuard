import {describe,expect,it,vi} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {DiagnosticScope} from './DiagnosticScope';
import type {DiagnosticFinding,DiagnosticReport} from '../core/types';

const report:DiagnosticReport={id:1,serviceOperation:2,generation:1,target:'192.0.2.1',interface:'en0',sourceAddress:'192.0.2.2',family:'IPv4',createdAt:1,declaredScope:{revision:1,purpose:'Web',environment:'production',owner:'Alex',exposure:'LAN only'},findings:[{id:'a',port:443,observedAt:1,category:'service',certainty:'observed'},{id:'b',port:22,observedAt:1,category:'service',certainty:'unverified'},{id:'c',port:443,observedAt:1,category:'tls',certainty:'observed'},{id:'d',port:443,observedAt:1,category:'http_headers',certainty:'not_performed'},{id:'e',port:22,observedAt:1,category:'http_redirect',certainty:'unverified'}] as DiagnosticFinding[]};
describe('exposure review',()=>{
 it('separates the saved declaration, source and observed service counts',()=>{
  const html=renderToStaticMarkup(<DiagnosticScope report={report} locale="ja" native selectedFinding="a" onSelectFinding={()=>{}}/>);
  expect(html).toContain('LAN only');
  expect(html).toContain('192.0.2.2');
  expect(html).toContain('応答を確認 1');
  expect(html).toContain('応答未確認 1');
  expect(html).toContain('番号からの候補');
  expect(html).toContain('TLS検証');
  expect(html).toContain('HTTPヘッダー');
  expect(html).toContain('未実施');
  expect(html).not.toContain('HTTPS誘導');
  expect(html).toContain('HTTPS');
  expect(html).toContain('SSH');
  expect(html).toContain('aria-pressed="true"');
  expect(html).toContain('aria-label="診断詳細を開く: 443');
  expect(html).toContain('aria-pressed="false"');
  expect(html).toContain('LAN内からの応答はインターネット公開を証明せず');
 });
 it('keeps the port selected when an evidence finding is selected',()=>{
  const html=renderToStaticMarkup(<DiagnosticScope report={report} locale="en" native selectedFinding="d" onSelectFinding={()=>{}}/>);
  expect(html).toContain('Open diagnostic details: 443 · HTTP headers · Not performed');
  expect(html).toContain('TLS verification');
  expect(html).not.toContain('HTTPS redirect');
 });
 it('shows the user-reported exposure review on the selected port',()=>{
  const reviewed={...report,exposureReviews:[{port:22,revision:1,status:'unexpected' as const,note:'Review SSH',reviewer:'Alex',reviewedAt:'2026-11-04'}]};
  const html=renderToStaticMarkup(<DiagnosticScope report={reviewed} locale="ja" native selectedFinding="b" onSelectFinding={()=>{}}/>);
  expect(html).toContain('申告: 想定外');
  expect(html).toContain('<span>想定外</span><strong>1</strong>');
  expect(html).toContain('<span>未確認</span><strong>1</strong>');
  expect(html).toContain('<span>想定内</span><strong>0</strong>');
  expect(html).toContain('公開意図の確認 · 22');
  expect(html).toContain('value="unexpected" selected');
  expect(html).toContain('value="Review SSH"');
  expect(html).toContain('value="Alex"');
  expect(html).toContain('value="2026-11-04"');
  expect(html).toContain('応答未確認 1');
 });
 it('counts due reviews from saved port records in each language',()=>{
  vi.useFakeTimers();vi.setSystemTime(new Date(2026,10,6,12));
  try{
   const reviewed={...report,exposureReviews:[{port:22,revision:1,status:'unexpected' as const,note:'',nextAt:'2026-11-05'},{port:443,revision:1,status:'expected' as const,note:'',nextAt:'2026-11-06'},{port:999,revision:1,status:'expected' as const,note:'',nextAt:'2026-11-01'}]};
   for(const [locale,today,overdue] of [['ja','本日確認','確認期限超過'],['en','Review today','Review overdue'],['zh-CN','今日确认','确认已逾期']] as const){
    const html=renderToStaticMarkup(<DiagnosticScope report={reviewed} locale={locale} native onSelectFinding={()=>{}}/>);
    expect(html).toContain(`<span>${today}</span><strong>1</strong>`);
    expect(html).toContain(`<span>${overdue}</span><strong>1</strong>`);
   }
  }finally{vi.useRealTimers();}
 });
 it('keeps missing declarations and sources unknown',()=>{
  const html=renderToStaticMarkup(<DiagnosticScope report={{...report,declaredScope:undefined,sourceAddress:undefined,findings:[]}} locale="en" native onSelectFinding={()=>{}}/>);
  expect(html).toContain('Not recorded');
  expect(html).toContain('Unknown');
  expect(html).toContain('Responses observed 0');
 });
});

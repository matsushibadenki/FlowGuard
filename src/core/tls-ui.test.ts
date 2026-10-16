import {describe,it,expect} from 'vitest';
import {validServiceHostname,tlsWords} from './tls-ui';
describe('HTTPS identity input',()=>{
 it('accepts optional ASCII DNS names while refusing destinations and header injection',()=>{
  for(const name of ['', ' Example.COM ', 'xn--bcher-kva.example','service.example.test'])expect(validServiceHostname(name),name).toBe(true);
  for(const name of ['https://example.com','127.0.0.1','::1','*.example.com','a..b','-a.b','a-.b','a.b.','a_b','例.example','x\r\nHost: evil','example.com\r\n','host:443','a'.repeat(64)])expect(validServiceHostname(name),name).toBe(false);
 });
 it('translates validation outcomes and missing evidence in all supported languages',()=>{
  for(const words of Object.values(tlsWords))for(const key of Object.keys(tlsWords.en))expect(words[key as keyof typeof words].length).toBeGreaterThan(0);
 });
});

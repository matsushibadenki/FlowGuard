import type {Connection, Rule} from './types';
export function ruleFor(rules:Rule[], c:Connection, source:string) {
  return rules.filter(r=>r.source===source && r.appKey===c.appKey && r.destination===c.destination && r.port===c.port && (r.direction==='both'||r.direction===c.direction)).sort((a,b)=>Number(b.direction!=='both')-Number(a.direction!=='both')||b.updatedAt-a.updatedAt)[0];
}
export function validRules(value:unknown): value is Rule[] {
  if(!Array.isArray(value)||value.length>2000) return false;
  const ids=new Set<string>();
  return value.every(r=> {
    if(!r || typeof r!=='object') return false;
    const valid=typeof r.id==='string'&&r.id.length<=8192&&typeof r.appKey==='string'&&r.appKey.length>0&&r.appKey.length<=4096&&typeof r.app==='string'&&r.app.length<=256&&typeof r.destination==='string'&&r.destination.length>0&&r.destination.length<=512&&Number.isInteger(r.port)&&r.port>0&&r.port<=65535&&['outbound','inbound','both'].includes(r.direction)&&['allow','block','ask'].includes(r.action)&&['live','demo'].includes(r.source)&&Number.isSafeInteger(r.updatedAt)&&r.updatedAt>=0&&!ids.has(r.id);
    ids.add(r.id);return valid;
  });
}
export function endpoint(c:Connection) {return `${c.destination.includes(':')?`[${c.destination}]`:c.destination}:${c.port}`;}

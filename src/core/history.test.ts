import {describe,it,expect} from 'vitest';
import {retainedHistory,validHistoryRetention,historyRetentions,historyWords} from './history';
import type {Connection} from './types';
const row=(observedAt:number)=>({id:String(observedAt),observedAt} as Connection);
describe('session history retention',()=>{
 it('uses a closed policy and defaults legacy preferences in callers',()=>{for(const p of historyRetentions)expect(validHistoryRetention(p)).toBe(true);for(const p of [null,undefined,0,'forever','900'])expect(validHistoryRetention(p)).toBe(false);});
 it('expires from first observation with an exclusive boundary and no future rows',()=>{expect(retainedHistory([row(1000),row(101),row(100),row(1001)],'15m',1000).map(c=>c.observedAt)).toEqual([1000,101]);expect(retainedHistory([row(1),row(2)],'1h',3601).map(c=>c.observedAt)).toEqual([2]);expect(retainedHistory([row(1),row(2)],'24h',86401).map(c=>c.observedAt)).toEqual([2]);});
 it('does not mutate observations and cannot restore deleted rows',()=>{const rows=[row(100),row(1000)];const next=retainedHistory(rows,'15m',1000);expect(rows).toHaveLength(2);expect(retainedHistory(next,'session',1000)).toEqual(next);expect(retainedHistory(rows,'off',1000)).toEqual([]);expect(retainedHistory([],'session',1000)).toEqual([]);});
 it('keeps the existing 500 row bound and order',()=>{const rows=Array.from({length:600},(_,i)=>row(600-i));expect(retainedHistory(rows,'session',1000)).toEqual(rows.slice(0,500));});
 it('supplies complete labels and limits in all three languages',()=>{for(const w of Object.values(historyWords)){expect(Object.keys(w)).toEqual(Object.keys(historyWords.en));expect(w.help).toContain('500');expect(w.count).toContain('{n}');expect(w.sessionOnly).toBeTruthy();}});
});

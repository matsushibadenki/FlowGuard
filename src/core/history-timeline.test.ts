import {describe,it,expect} from 'vitest';
import {historyTimeline,inHistoryRange} from './history-timeline';
import type {Connection} from './types';
const row=(observedAt:number)=>({observedAt} as Connection);
describe('retained history distribution',()=>{
 it('keeps exact interval boundaries and empty gaps',()=>{
  const buckets=historyTimeline([row(60),row(119),row(180)]);
  expect(buckets.map(b=>b.count)).toEqual([2,0,1]);
  expect(inHistoryRange(row(120),buckets[0])).toBe(false);
  expect(inHistoryRange(row(60),buckets[0])).toBe(true);
 });
 it('bounds output for a long retained session without dropping records',()=>{
  const buckets=historyTimeline([row(0),row(1e9)]);
  expect(buckets.length).toBeLessThanOrEqual(12);
  expect(buckets.reduce((sum,b)=>sum+b.count,0)).toBe(2);
 });
 it('does not invent observations for empty or invalid history',()=>{
  expect(historyTimeline([])).toEqual([]);
  expect(historyTimeline([row(NaN),row(-1),row(Infinity)])).toEqual([]);
  expect(inHistoryRange(row(1),null)).toBe(true);
 });
});

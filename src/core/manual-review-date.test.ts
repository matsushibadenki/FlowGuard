import {describe,expect,it} from 'vitest';
import {localReviewDate,manualReviewDue} from './manual-review-date';
describe('manual review dates',()=>{
 it('distinguishes overdue, today and future dates using local calendar days',()=>{
  const entry={status:'confirmed',nextAt:'2026-11-02'};
  expect(manualReviewDue(entry,'2026-11-03')).toBe('overdue');
  expect(manualReviewDue(entry,'2026-11-02')).toBe('today');
  expect(manualReviewDue(entry,'2026-11-01')).toBeNull();
  expect(localReviewDate(new Date(2026,10,2,23,59))).toBe('2026-11-02');
 });
 it('does not treat missing, invalid, unknown or out-of-scope entries as scheduled',()=>{
  expect(manualReviewDue(undefined,'2026-11-02')).toBeNull();
  for(const nextAt of ['', '2026-02-29','2026-13-01','0000-01-01'])expect(manualReviewDue({status:'confirmed',nextAt},'2026-11-02')).toBeNull();
  for(const status of ['unknown','out_of_scope','future'])expect(manualReviewDue({status,nextAt:'2020-01-01'},'2026-11-02')).toBeNull();
  expect(manualReviewDue({status:'needs_work',nextAt:'2024-02-29'},'2026-11-02')).toBe('overdue');
 });
});

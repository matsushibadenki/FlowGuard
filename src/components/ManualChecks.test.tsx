import {describe, expect, it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {ManualChecks} from './ManualChecks';
import type {DiagnosticReport} from '../core/types';

const report: DiagnosticReport = {
 id:1, serviceOperation:1, generation:1, target:'192.0.2.1', interface:'en0', createdAt:1, findings:[],
 manualChecks:[{item:'mfa', status:'confirmed', evidence:'Reviewed settings', owner:'Alex', checkedAt:'2026-10-29', nextAt:'2027-01-29'}],
};

describe('manual checks',()=>{
 it('restores the selected MFA entry when a report is opened again',()=>{
  const markup=renderToStaticMarkup(<ManualChecks report={report} locale="ja" native={true}/>);
  expect(markup).toContain('value="Reviewed settings"');
  expect(markup).toContain('value="Alex"');
  expect(markup).toContain('value="2026-10-29"');
  expect(markup).toContain('value="2027-01-29"');
  expect(markup).toContain('value="confirmed" selected');
 });
 it('starts unconfirmed when the report has no saved MFA entry',()=>{
  const markup=renderToStaticMarkup(<ManualChecks report={{...report, manualChecks:[]}} locale="en" native={true}/>);
  expect(markup).toContain('value="unknown" selected');
  expect(markup).not.toContain('Reviewed settings');
 });
});

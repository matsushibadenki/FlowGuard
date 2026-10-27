import {describe,expect,it} from 'vitest';
import {diagnosticCounts,diagnosticDispositions,diagnosticChanges} from './diagnostics';
import type {DiagnosticFinding} from './types';

describe('diagnostic overview',()=>{
 it('counts evidence state and severity independently without treating unperformed as safe',()=>{
  const rows=[{port:443,certainty:'observed',severity:'warning'},{port:443,certainty:'not_performed',severity:'info'},{port:22,certainty:'unverified',severity:'info'},{port:22,certainty:'future',severity:'future'}] as DiagnosticFinding[];
  expect(diagnosticCounts(rows)).toEqual({certainty:{observed:1,unverified:1,not_performed:1,other:1},severity:{warning:1,info:2,other:1},ports:2});
  expect(rows).toHaveLength(4);
 });
 it('counts user dispositions across the report, including unknown values',()=>{const rows=[{disposition:'open'},{disposition:'resolved'},{disposition:'resolved'},{disposition:'recheck'},{disposition:'out_of_scope'},{disposition:'future'}] as DiagnosticFinding[];expect(diagnosticDispositions(rows)).toEqual({open:1,resolved:2,recheck:1,out_of_scope:1,other:1});expect(diagnosticDispositions([])).toEqual({open:0,resolved:0,recheck:0,out_of_scope:0,other:0});});
 it('counts comparison states without confusing uncertain or absent comparisons with unchanged',()=>{
  const rows=[{change:'changed'},{change:'new'},{change:'unchanged'},{change:'uncertain'},{change:'uncertain'},{change:'uncompared'},{change:'future'}] as DiagnosticFinding[];
  expect(diagnosticChanges(rows)).toEqual({changed:1,new:1,unchanged:1,uncertain:2,uncompared:1,other:1});
  expect(diagnosticChanges([])).toEqual({changed:0,new:0,unchanged:0,uncertain:0,uncompared:0,other:0});
 });
 it('shows zero counts for an empty selected report',()=>expect(diagnosticCounts([])).toEqual({certainty:{observed:0,unverified:0,not_performed:0,other:0},severity:{warning:0,info:0,other:0},ports:0}));
});

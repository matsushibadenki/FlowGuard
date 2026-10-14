import {describe, expect, it} from 'vitest';
import {recoveryState} from './recovery';
import {defaultExtension} from './enforcement';
import type {Snapshot} from './types';
const fixture = (stage: string, changes = {}) => ({extension: {...defaultExtension, stage, signed: true, packaged: true, runtimeAvailable: true, ...changes}, updates: {stage: 'idle'}} as Pick<Snapshot, 'extension' | 'updates'>);
describe('emergency recovery evidence and admission', () => {
  it('remains available when disconnected or in demo; no provider connection is needed', () => {
    expect(recoveryState(fixture('disconnected'), true).canDisable).toBe(true);
    expect(recoveryState(fixture('failed'), true).canDisable).toBe(true);
  });
  it('never claims settings confirmation from missing, unsigned, busy or failed evidence', () => {
    for (const stage of ['disconnected', 'inactive', 'failed', 'disabling', 'restart_required', 'configuration_conflict']) expect(recoveryState(fixture(stage), true).confirmed).toBe(false);
    expect(recoveryState(fixture('disabled', {signed: false}), true).confirmed).toBe(false);
    expect(recoveryState(fixture('disabled', {busy:true}), true).confirmed).toBe(false);
    expect(recoveryState(fixture('disabled'), true)).toMatchObject({confirmed: true, canDisable: false});
  });
  it('blocks previews, missing runtime, concurrent extension operations and installing updates', () => {
    expect(recoveryState(fixture('applied'), false).canDisable).toBe(false);
    for (const change of [{signed: false}, {packaged: false}, {runtimeAvailable: false}]) expect(recoveryState(fixture('applied', change), true).canDisable).toBe(false);
    expect(recoveryState(fixture('applying', {busy:true}),true).canDisable).toBe(true);
    expect(recoveryState(fixture('disabling', {busy:true}),true).canDisable).toBe(false);
    const s = fixture('applied'); s.updates.stage = 'installing'; expect(recoveryState(s, true).canDisable).toBe(false);
  });
});

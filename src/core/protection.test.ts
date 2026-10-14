// パス: src/core/protection.test.ts
import {describe, expect, it} from 'vitest';
import type {Snapshot} from './types';
import {defaultExtension} from './enforcement';
import {protectionState} from './protection';
const make = (stage = 'ready', mode = 'live', error: string | null = null) => ({
  document: {preferences: {mode}, rules: []}, firewall: {error},
  extension: {...defaultExtension, stage, signed: true, instanceId: 'boot', providerRevision: 1},
} as unknown as Snapshot);
describe('map protection states', () => {
  it('prioritizes demo and unavailable observation over acknowledgement', () => {
    expect(protectionState(make('applied', 'demo')).steps).toEqual([false, false, false, false]);
    expect(protectionState(make('applied', 'live', 'error')).headline).toBe('protectionObserveFailed');
  });
  it('requires an idle signed receipt and labels uncertainty', () => {
    expect(protectionState(make('applied')).steps).toEqual([true, true, true, true]);
    for (const stage of ['stale', 'disconnected', 'failed', 'applying', 'initial_sync', 'disabling', 'removing']) {
      expect(protectionState(make(stage)).headline).toBe('mapUncertainHeadline');
    }
    const s = make('applied'); s.extension.busy = true;
    expect(protectionState(s).tone).toBe('caution');
    s.extension.busy = false; s.extension.signed = false;
    expect(protectionState(s).headline).toBe('mapUncertainHeadline');
  });
  it('counts independent states instead of claiming continuous progress', () => {
    const state = protectionState(make());
    expect(state.steps).toEqual([true, false, true, false]);
    expect(state.reached).toBe(2);
  });
});

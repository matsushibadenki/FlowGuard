// パス: src/core/protection.ts
import type {Snapshot} from './types';
import type {TextKey} from './i18n';
export type ProtectionTone = 'neutral' | 'caution' | 'acknowledged';
export interface ProtectionState {
  headline: TextKey;
  tone: ProtectionTone;
  reached: 0 | 1 | 2 | 3 | 4;
  steps: [boolean, boolean, boolean, boolean];
}
export function protectionState(snapshot: Snapshot): ProtectionState {
  const {extension, firewall, document} = snapshot;
  let headline: TextKey = 'protectionObserveOnly';
  let tone: ProtectionTone = 'neutral';
  let steps: ProtectionState['steps'] = [false, false, false, false];
  if (document.preferences.mode === 'demo') headline = 'mapDemoHeadline';
  else if (firewall.error !== null) {headline = 'protectionObserveFailed'; tone = 'caution';}
  else if (extension.stage === 'applied' && !extension.busy && extension.signed &&
           extension.instanceId !== null && extension.providerRevision !== null) {
    headline = 'protectionAcknowledged'; tone = 'acknowledged'; steps = [true, true, true, true];
  } else {
    const uncertain = extension.busy || ['stale', 'disconnected', 'failed', 'applying',
      'initial_sync', 'disabling', 'removing', 'configuration_conflict', 'applied'].includes(extension.stage);
    if (uncertain) {headline = 'mapUncertainHeadline'; tone = 'caution';}
    steps = [true, document.rules.some(r => r.source === 'live'),
      !uncertain && extension.stage === 'ready' && extension.signed, false];
  }
  // Count independent confirmed states; this is not a continuous progress percentage.
  return {headline, tone, steps, reached: steps.filter(Boolean).length as ProtectionState['reached']};
}

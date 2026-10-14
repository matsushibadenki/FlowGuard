import {Info, Unplug} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {recoveryState} from '../core/recovery';
import {disableExtension, native} from '../core/store';
import type {T} from './ui';
export function EmergencyRecovery({snapshot, t, onError}: {snapshot: Snapshot; t: T; onError: (error: unknown) => void}) {
  const state = recoveryState(snapshot, native);
  return <section className="emergency-recovery surface" aria-label={t('emergencyRecovery')}>
    <div className="recovery-action"><Unplug size={18}/><strong>{t('emergencyRecovery')}</strong>
      <span className="protection-info" tabIndex={0} title={t('emergencyRecoveryHelp')} aria-label={t('emergencyRecoveryHelp')}><Info size={16}/></span>
      <button className="secondary-button" disabled={!state.canDisable}
        onClick={() => void disableExtension().catch(onError)}>{t(state.pending ? 'recoveryDisabling' : 'recoveryDisable')}</button>
    </div>
    <dl><div><dt>{t('recoveryOS')}</dt><dd>{t(state.confirmed ? 'recoveryOSSaved' : state.pending ? 'recoveryDisabling' : 'stateUnconfirmed')}</dd></div>
      <div><dt>{t('recoveryProvider')}</dt><dd>{t('stateUnconfirmed')}</dd></div>
      <div><dt>{t('recoveryTraffic')}</dt><dd>{t('stateUnconfirmed')}</dd></div></dl>
    {state.confirmed && <p role="status">{t('recoveryCheckTraffic')}</p>}
    {(!native || !snapshot.extension.signed) && <small>{t('recoveryDesktopRequired')}</small>}
    {snapshot.extension.busy && <small>{t('recoveryWaitOperation')}</small>}
    {snapshot.updates.stage === 'installing' && <small>{t('recoveryWaitUpdate')}</small>}
  </section>;
}

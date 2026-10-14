import type {Snapshot} from './types';
// OS preference acknowledgement is the only available recovery evidence.
// An absent XPC connection is never proof of Provider shutdown or restored traffic.
export function recoveryState(snapshot: Pick<Snapshot, 'extension' | 'updates'>, desktop: boolean) {
  const {extension: status, updates} = snapshot;
  const confirmed = status.signed && !status.busy && ['disabled', 'removed'].includes(status.stage);
  const pending = status.busy && status.stage === 'disabling';
  const canDisable = desktop && status.packaged && status.runtimeAvailable && status.signed &&
    (!status.busy || status.stage !== 'disabling') && updates.stage !== 'installing' && !confirmed;
  return {confirmed, pending, canDisable};
}

// パス: src/components/ProtectionStatus.tsx
import {Info} from 'lucide-react';
import type {Snapshot} from '../core/types';
import {protectionState} from '../core/protection';
import {dispatch} from '../core/store';
import type {T} from './ui';
export function ProtectionStatus({snapshot, t, native, onError}: {
  snapshot: Snapshot; t: T; native: boolean; onError: (error: unknown) => void;
}) {
  const state = protectionState(snapshot);
  const mode = snapshot.document.preferences.mode;
  const steps = ['stepObserve', 'stepIntent', 'stepExtension', 'stepAcknowledged'] as const;
  return <section className={`map-protection surface tone-${state.tone}`} aria-label={t('status')}>
    <div className="map-protection-copy"><strong role="status" aria-live="polite">{state.tone === 'caution' && <Info size={18}/>}<span className="map-headline-parts">{t(state.headline).split(' — ').map((part, index) => <span key={part}>{index > 0 ? '— ' : ''}{part}</span>)}</span></strong>
      {state.tone !== 'acknowledged' && <details><summary><Info size={14}/>{t('stepHelp')}</summary><p>{t('protectionHelp')}</p></details>}
    </div>
    <ol className="map-protection-track" aria-label={t('stepsConfirmed').replace('{n}', String(state.reached))}>
      {steps.map((step, index) => <li key={step} className={state.steps[index] ? 'confirmed' : ''}
        title={`${t(step)}: ${state.steps[index] ? t('stateConfirmed') : t('stateUnconfirmed')}`}>
        <span className="map-step-dot" aria-hidden="true"/><span className="map-step-label">{t(step).split(/(?=[(（])/).map(part => <span key={part}>{part}</span>)}</span>
        <span className="sr-only">{state.steps[index] ? t('stateConfirmed') : t('stateUnconfirmed')}</span>
      </li>)}
    </ol>
    <div className="map-protection-numbers"><div className="source-switch">
      {(['live', 'demo'] as const).map(value => <button key={value} aria-pressed={mode === value} disabled={value === 'live' && !native}
        onClick={() => void dispatch({type: 'setMode', mode: value}).catch(onError)}>{t(value)}</button>)}
    </div><dl>{[[t('connections'), snapshot.connections.length], [t('appsShort'), snapshot.applications.length],
      [t('stepIntent'), snapshot.document.rules.filter(r => r.source === mode).length]].map(([label, value]) =>
      <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl></div>
  </section>;
}

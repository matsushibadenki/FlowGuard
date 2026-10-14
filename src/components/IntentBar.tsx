// パス: src/components/IntentBar.tsx
import type {Intent} from '../core/mapModel';
import type {T} from './ui';
export function IntentBar({counts, t}: {counts: Record<Intent, number>; t: T}) {
  const total = Object.values(counts).reduce((sum, n) => sum + n, 0);
  return <span className="map-intent-bar" role="img" aria-label={(['allow', 'block', 'ask', 'none'] as const)
    .map(intent => `${t(intent === 'none' ? 'noRule' : `${intent}Intent`)} ${counts[intent]}`).join(', ')}>
    {(['allow', 'block', 'ask', 'none'] as const).map(intent =>
      <span key={intent} className={`intent-${intent}`} style={{width: `${total ? counts[intent] / total * 100 : 0}%`}}/>)}</span>;
}

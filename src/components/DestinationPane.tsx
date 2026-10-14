// パス: src/components/DestinationPane.tsx
import {Check, ChevronDown, ChevronRight, Info} from 'lucide-react';
import type {AppRow, DestinationRow} from '../core/mapModel';
import type {DestinationClass} from '../core/destinations';
import {ActionIcon, AppIcon, DirectionIcon, type T} from './ui';
const classes: DestinationClass[] = ['public', 'local', 'other', 'loopback'];
const labels = {public: 'classPublic', local: 'classLocal', other: 'classOther', loopback: 'classLoopback'} as const;
export function DestinationPane({row, selectedId, select, t, expanded, limits, toggle, showMore, hiddenConnections, showHidden}: {
  row: AppRow | undefined; selectedId: string | null; select: (row: DestinationRow) => void; t: T;
  expanded: Record<string, boolean>; limits: Record<string, number>;
  toggle: (key: string, current: boolean) => void; showMore: (key: string, current: number) => void;
  hiddenConnections: number; showHidden: () => void;
}) {
  return <section className="map-destination-pane surface" aria-label={t('destinations')}>
    {!row ? <div className="empty"><p>{t('pickApp')}</p></div> : <>
      <header className="map-selected-app"><AppIcon app={row.app}/><div><h2 title={row.app}>{row.app}</h2>
        <small className="map-app-path" title={row.appKey}>{row.appKey || t('noIdentity')}</small>
        <p>{t('mapConnectionSummary').replace('{n}', String(row.connectionCount)).replace('{m}', String(row.processCount))}</p>
      </div></header>
      <div className="map-intent-legend">{(['allow', 'block', 'ask'] as const).map(intent =>
        <span key={intent} className={`intent-text-${intent}`}><ActionIcon action={intent}/>{t(`${intent}Intent`)}</span>)}</div>
      <details className="map-class-help"><summary><Info size={15}/>{t('classRangeLabel')}</summary><p>{t('classHelp')}</p></details>
      {classes.map(klass => {
        const destinations = row.destinations.filter(d => d.klass === klass);
        if (!destinations.length) return null;
        const key = `${row.id}:${klass}`;
        const open = expanded[key] ?? klass !== 'loopback';
        const limit = limits[key] ?? 50;
        const shown = destinations.slice(0, limit);
        return <section className="map-destination-group" key={key}>
          <h3><button aria-expanded={open} aria-controls={`destinations-${encodeURIComponent(key)}`} onClick={() => toggle(key, open)}>
            {open ? <ChevronDown size={16}/> : <ChevronRight size={16}/>}{t(labels[klass])} ({destinations.length})</button></h3>
          {open && <div id={`destinations-${encodeURIComponent(key)}`}><ul>{shown.map(destination => {
            const selected = destination.connectionIds.includes(selectedId ?? '');
            return <li key={destination.key}><button className={`map-destination-row intent-${destination.intent}`}
              aria-pressed={selected} onClick={() => select(destination)}>
              <span className="map-endpoint"><strong>{destination.destination.includes(':') ? `[${destination.destination}]` : destination.destination}:{destination.port}</strong>
                <span><span>{destination.protocol}</span><DirectionIcon direction={destination.direction}/>
                  {t(destination.direction === 'unknown' ? 'unknownDirection' : destination.direction === 'inbound' ? 'inbound' : 'outbound')}</span>
              </span><span className="map-intent-chip">
                {destination.intent === 'none' ? t('noRule') : <><span className={`intent-text-${destination.intent}`}><ActionIcon action={destination.intent}/>{t(`${destination.intent}Intent`)}</span>
                  {destination.acknowledged && <small><Check size={13}/>{t('applied')}</small>}</>}
              </span>{destination.connectionIds.length > 1 && <span className="map-merged">{t('mergedConnections').replace('{n}', String(destination.connectionIds.length))}</span>}
            </button></li>;
          })}</ul><p className="map-row-count">{t('mapRowsShown').replace('{n}', String(shown.length)).replace('{m}', String(destinations.length))}</p>
          {destinations.length > limit && <button className="secondary-button map-more" onClick={() => showMore(key, limit)}>
            {t('showMore').replace('{n}', String(Math.min(50, destinations.length - limit)))}</button>}</div>}
        </section>;
      })}
      {hiddenConnections > 0 && <div className="map-hidden-apps"><p>{t('mapHiddenConnections').replace('{n}', String(hiddenConnections))}</p>
        <button className="secondary-button" onClick={showHidden}>{t('showHidden')}</button></div>}
    </>}
  </section>;
}

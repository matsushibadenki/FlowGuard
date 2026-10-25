// パス: src/components/AppList.tsx
import {useRef, useState} from 'react';
import type {AppRow} from '../core/mapModel';
import {AppIcon, type T} from './ui';
import {IntentBar} from './IntentBar';
export function AppList({rows, activeId, select, t, hiddenApps, showHidden}: {
  rows: AppRow[]; activeId: string | undefined; select: (row: AppRow) => void;
  t: T; hiddenApps: number; showHidden: () => void;
}) {
  const [focusId, setFocusId] = useState<string | null>(null);
  const buttons = useRef(new Map<string, HTMLButtonElement>());
  const tabId = rows.some(row => row.id === focusId) ? focusId : activeId ?? rows[0]?.id;
  function move(index: number, key: string) {
    const next = key === 'Home' ? 0 : key === 'End' ? rows.length - 1 :
      (index + (key === 'ArrowDown' ? 1 : -1) + rows.length) % rows.length;
    setFocusId(rows[next].id); buttons.current.get(rows[next].id)?.focus();
  }
  return <section className="map-app-list surface" aria-label={t('appsShort')}>
    <h2>{t('appsShort')} ({rows.length})</h2>
    <ul>{rows.map((row, index) => {
      const count = row.connectionCount - row.intentCounts.none;
      return <li key={row.id}><button className="map-app-row" aria-current={row.id === activeId ? 'true' : undefined}
        tabIndex={row.id === tabId ? 0 : -1} ref={node => {if (node) buttons.current.set(row.id, node); else buttons.current.delete(row.id);}}
        onFocus={() => setFocusId(row.id)} onClick={() => select(row)}
        onKeyDown={event => {if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {event.preventDefault(); move(index, event.key);}}}>
        <AppIcon app={row.app} appKey={row.appKey} small/><span className="map-app-copy"><strong title={row.app}>{row.app}</strong>
          <span className="map-app-intents">{count > 0 ? <><IntentBar counts={row.intentCounts} t={t}/>
            {t('intentCount').replace('{n}', String(count)).replace('{m}', String(row.connectionCount))}</> : t('noRule')}</span>
        </span><span className="map-app-count" aria-label={`${row.connectionCount} ${t('connections')}`}>{row.connectionCount}</span>
      </button></li>;
    })}</ul>
    {hiddenApps > 0 && <div className="map-hidden-apps"><p>{t('hiddenApps').replace('{n}', String(hiddenApps))}</p>
      <button className="secondary-button" onClick={showHidden}>{t('showHidden')}</button></div>}
  </section>;
}

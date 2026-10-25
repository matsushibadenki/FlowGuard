// パス: src/components/FlowMap.tsx
import type {GpuFilter} from '../core/gpu-panel';
import {GpuMap} from './GpuMap';
import {lazy, Suspense, useMemo, useState} from 'react';
import type {Connection, Snapshot} from '../core/types';
import {buildAppRows, representativeConnection, type AppRow, type SortKey} from '../core/mapModel';
import {classifyAddress} from '../core/destinations';
import {AppList} from './AppList';
import {DestinationPane} from './DestinationPane';
import {Empty, type T} from './ui';
import {Network, List} from 'lucide-react';
const TrafficGraph=lazy(()=>import('./TrafficGraph').then(m=>({default:m.TrafficGraph})));
export function FlowMap({snapshot, connections, t, select, clearFilters, hideLoopback, setHideLoopback, sort, gpuFilter}: {
  gpuFilter:GpuFilter; snapshot: Snapshot; connections: Connection[]; t: T; select: (c: Connection) => void; clearFilters?: () => void; hideLoopback: boolean; setHideLoopback: (value: boolean) => void; sort: SortKey;
}) {
  const [view,setView]=useState<'graph'|'list'>('graph');
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [limits, setLimits] = useState<Record<string, number>>({});
  const model = useMemo(() => buildAppRows({groups: snapshot.applications, connections,
    rules: snapshot.document.rules, mode: snapshot.document.preferences.mode, extension: snapshot.extension,
    hideLoopback, sort, locale: snapshot.document.preferences.locale}),
  [snapshot.applications, connections, snapshot.document.rules, snapshot.document.preferences.mode,
    snapshot.document.preferences.locale, snapshot.extension, hideLoopback, sort]);
  const byId = useMemo(() => new Map(connections.map(c => [c.id, c])), [connections]);
  const selectedGroup = snapshot.applications.find(group => group.connectionIds.includes(snapshot.selectedId ?? ''));
  const active = model.rows.find(row => row.id === selectedGroup?.id) ?? model.rows[0];
  const activeIds = new Set(snapshot.applications.find(group => group.id === active?.id)?.connectionIds ?? []);
  const hiddenConnections = hideLoopback ? connections.filter(c =>
    activeIds.has(c.id) && classifyAddress(c.destination) === 'loopback').length : 0;
  function selectApp(row: AppRow) {
    const c = representativeConnection(row.destinations.flatMap(d => d.connectionIds.flatMap(id => {
      const c = byId.get(id); return c ? [c] : [];
    })));
    if (c) select(c);
  }
  return <section className="flowmap-redesign" aria-label={t('map')}>
    <GpuMap snapshot={snapshot} filter={gpuFilter}/>
    {snapshot.connections.length === 0 ? <div className="surface"><Empty t={t}/></div> : <>
      {model.rows.length === 0 && <div className="map-no-matches surface"><p>{t('noMatches')}</p>
        {clearFilters && <button className="secondary-button" onClick={clearFilters}>{t('clearFilters')}</button>}</div>}
      <div className="map-graph-switch" role="group" aria-label={t('map')}><button aria-pressed={view==='graph'} onClick={()=>setView('graph')}><Network size={18}/>{t('graphView')}</button><button aria-pressed={view==='list'} onClick={()=>setView('list')}><List size={18}/>{t('listView')}</button></div>
      {view==='graph' ? <>{model.rows.length>0&&<Suspense fallback={<p role="status">{t('loading')}</p>}><TrafficGraph key={JSON.stringify([gpuFilter.search,gpuFilter.intent,hideLoopback,sort])} snapshot={snapshot} rows={model.rows} t={t} selectApp={selectApp} selectDestination={row=>{const c=byId.get(row.representativeId);if(c)select(c);}}/></Suspense>}{model.hiddenLoopbackOnlyApps>0&&<div className="map-hidden-apps"><p>{t('hiddenApps').replace('{n}',String(model.hiddenLoopbackOnlyApps))}</p><button className="secondary-button" onClick={()=>setHideLoopback(false)}>{t('showHidden')}</button></div>}</> : <div className="map-panes"><AppList rows={model.rows} activeId={active?.id} select={selectApp} t={t}
        hiddenApps={model.hiddenLoopbackOnlyApps} showHidden={() => setHideLoopback(false)}/>
        <DestinationPane snapshot={snapshot} row={active} selectedId={snapshot.selectedId} t={t} expanded={expanded} limits={limits}
          select={row => {const c = byId.get(row.representativeId); if (c) select(c);}}
          toggle={(key, current) => setExpanded(value => ({...value, [key]: !current}))}
          showMore={(key, current) => setLimits(value => ({...value, [key]: current + 50}))}
          hiddenConnections={hiddenConnections} showHidden={() => setHideLoopback(false)}/>
      </div>}
    </>}
  </section>;
}

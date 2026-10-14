// パス: src/core/mapModel.ts
import type {ApplicationGroup, Connection, ExtensionStatus, Rule} from './types';
import {classifyAddress, type DestinationClass} from './destinations';
import {ruleFor} from './rules';
import {ruleApplied} from './enforcement';
export type Intent = 'allow' | 'block' | 'ask' | 'none';
export interface DestinationRow {
  key: string; destination: string; port: number; protocol: string; direction: string;
  klass: DestinationClass; connectionIds: string[]; representativeId: string;
  intent: Intent; acknowledged: boolean;
}
export interface AppRow {
  id: string; app: string; appKey: string; processCount: number; connectionCount: number;
  byClass: Record<DestinationClass, number>; intentCounts: Record<Intent, number>;
  attention: number; destinations: DestinationRow[];
}
export type SortKey = 'connections' | 'name' | 'attention';
export function representativeConnection(connections: Connection[]): Connection | undefined {
  return [...connections].sort((a, b) =>
    Number(classifyAddress(a.destination) === 'loopback') - Number(classifyAddress(b.destination) === 'loopback') ||
    b.observedAt - a.observedAt || a.id.localeCompare(b.id))[0];
}
export function buildAppRows(args: {
  groups: ApplicationGroup[]; connections: Connection[]; rules: Rule[];
  mode: 'live' | 'demo'; extension: ExtensionStatus; hideLoopback: boolean;
  sort: SortKey; locale: string;
}): {rows: AppRow[]; hiddenLoopbackOnlyApps: number} {
  const visible = new Map(args.connections.map(c => [c.id, c]));
  const rows: AppRow[] = [];
  let hiddenLoopbackOnlyApps = 0;
  const classOrder = {public: 0, local: 1, other: 2, loopback: 3};
  for (const group of args.groups) {
    const all = [...new Set(group.connectionIds)].flatMap(id => {
      const c = visible.get(id); return c ? [c] : [];
    });
    if (!all.length) continue;
    const connections = all.filter(c => !args.hideLoopback || classifyAddress(c.destination) !== 'loopback');
    if (!connections.length) {hiddenLoopbackOnlyApps++; continue;}
    const row: AppRow = {id: group.id, app: group.app, appKey: group.appKey,
      processCount: new Set(connections.map(c => c.pid)).size, connectionCount: connections.length,
      byClass: {public: 0, local: 0, other: 0, loopback: 0},
      intentCounts: {allow: 0, block: 0, ask: 0, none: 0}, attention: 0, destinations: []};
    const destinations = new Map<string, DestinationRow>();
    for (const c of connections) {
      const klass = classifyAddress(c.destination);
      const rule = ruleFor(args.rules, c, args.mode) as Rule | undefined;
      const intent = rule?.action ?? 'none';
      row.byClass[klass]++; row.intentCounts[intent]++;
      if (klass === 'public' && intent === 'none') row.attention++;
      const key = `${c.destination}|${c.port}|${c.protocol}|${c.direction}`;
      const previous = destinations.get(key);
      if (previous) {
        previous.connectionIds.push(c.id);
        const representative = visible.get(previous.representativeId)!;
        if (c.observedAt > representative.observedAt ||
            (c.observedAt === representative.observedAt && c.id < representative.id)) previous.representativeId = c.id;
      } else destinations.set(key, {key, destination: c.destination, port: c.port,
        protocol: c.protocol, direction: c.direction, klass, connectionIds: [c.id],
        representativeId: c.id, intent, acknowledged: !!rule && ruleApplied(rule, args.extension)});
    }
    row.destinations = [...destinations.values()].sort((a, b) => classOrder[a.klass] - classOrder[b.klass] ||
      b.connectionIds.length - a.connectionIds.length || a.destination.localeCompare(b.destination) ||
      a.port - b.port || a.key.localeCompare(b.key));
    rows.push(row);
  }
  rows.sort((a, b) => (args.sort === 'attention' ? b.attention - a.attention || b.connectionCount - a.connectionCount :
    args.sort === 'connections' ? b.connectionCount - a.connectionCount : 0) ||
    a.app.localeCompare(b.app, args.locale) || a.id.localeCompare(b.id));
  return {rows, hiddenLoopbackOnlyApps};
}

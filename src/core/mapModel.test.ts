// パス: src/core/mapModel.test.ts
import {describe, expect, it} from 'vitest';
const signing = {authority:'developer' as const,teamId:'ABCDE12345',identifier:'com.test.app'};
const subject = 'signed:v1:developer:ABCDE12345:com.test.app';
import {buildAppRows, representativeConnection} from './mapModel';
import {groupApplications} from './applications';
import {defaultExtension} from './enforcement';
import type {Connection, Rule} from './types';
const c = (id: string, destination = '150.171.110.134', overrides: Partial<Connection> = {}): Connection => ({
  identity: signing, id, app: 'App', appKey: '/app', pid: 1, local: '127.0.0.1:1234', destination,
  port: 443, protocol: 'TCP', direction: 'unknown', observedAt: 1, ...overrides,
});
const build = (connections: Connection[], rules: Rule[] = [], hideLoopback = true) => buildAppRows({
  groups: groupApplications(connections), connections, rules, hideLoopback,
  mode: 'live', extension: defaultExtension, sort: 'connections', locale: 'en',
});
describe('application destination projection', () => {
  it('merges sockets, preserves counts and chooses the newest representative', () => {
    const {rows} = build([c('a'), c('b', undefined, {pid: 2, observedAt: 3})]);
    expect(rows).toHaveLength(1); expect(rows[0].processCount).toBe(2);
    expect(rows[0].connectionCount).toBe(2); expect(rows[0].intentCounts.none).toBe(2);
    expect(rows[0].destinations[0].connectionIds).toEqual(['a', 'b']);
    expect(rows[0].destinations[0].representativeId).toBe('b');
  });
  it('hides loopback-only apps and never merges unknown processes by name', () => {
    const connections = [c('a', '127.0.0.1'), c('b', '::1', {appKey: '/other'}),
      c('x', undefined, {appKey: '', pid: 3}), c('y', undefined, {appKey: '', pid: 4})];
    const result = build(connections);
    expect(result.hiddenLoopbackOnlyApps).toBe(2); expect(result.rows).toHaveLength(2);
    expect(build(connections, [], false).rows).toHaveLength(4);
  });
  it('drops missing and duplicate IDs and has no six-app limit', () => {
    const connections = Array.from({length: 172}, (_, i) => c(String(i), undefined, {appKey: `/app-${i}`}));
    const groups = groupApplications(connections);
    groups[0].connectionIds.push('missing', groups[0].connectionIds[0]);
    const result = buildAppRows({groups, connections, rules: [], mode: 'live', extension: defaultExtension,
      hideLoopback: true, sort: 'name', locale: 'en'});
    expect(result.rows).toHaveLength(172);
    expect(result.rows.reduce((n, row) => n + row.connectionCount, 0)).toBe(172);
  });
  it('preserves both fallback, exact direction priority and unknown direction', () => {
    const rule: Rule = {identity: signing, id: 'both', app: 'App', appKey: subject, destination: '150.171.110.134',
      port: 443, direction: 'both', action: 'block', source: 'live', updatedAt: 2};
    const rules = [rule, {...rule, id: 'out', direction: 'outbound' as const, action: 'allow' as const, updatedAt: 1}];
    const {rows} = build([c('u'), c('o', undefined, {direction: 'outbound'})], rules);
    expect(rows[0].intentCounts).toEqual({block: 1, allow: 1, ask: 0, none: 0});
    expect(rows[0].destinations).toHaveLength(2);
    expect(rows[0].destinations.every(d => !d.acknowledged)).toBe(true);
  });
  it('prefers a non-loopback selection even when loopback is newer', () => {
    expect(representativeConnection([c('loop', '::1', {observedAt: 20}), c('net')])?.id).toBe('net');
  });
});

describe('map sorting and acknowledgement', () => {
  it('sorts unruled public counts without labeling them as threats', () => {
    const connections = [c('a', '192.168.1.1', {appKey: '/a', app: 'Alpha'}),
      c('b', '150.171.110.134', {appKey: '/b', app: 'Beta'})];
    const common = {groups: groupApplications(connections), connections, rules: [], mode: 'live' as const,
      extension: defaultExtension, hideLoopback: true, locale: 'en'};
    expect(buildAppRows({...common, sort: 'attention'}).rows.map(r => r.app)).toEqual(['Beta', 'Alpha']);
    expect(buildAppRows({...common, sort: 'name'}).rows.map(r => r.app)).toEqual(['Alpha', 'Beta']);
  });
  it('uses existing rule receipt and isolates live and demo rules', () => {
    const connection = c('a');
    const rule: Rule = {identity: signing, id: 'r', app: 'App', appKey: subject, destination: connection.destination,
      port: 443, direction: 'both', action: 'ask', source: 'live', updatedAt: 1};
    const args = {groups: groupApplications([connection]), connections: [connection], rules: [rule],
      mode: 'live' as const, extension: {...defaultExtension, stage: 'applied', signed:true, instanceId:'provider', providerRevision:1, appliedRuleIds: ['r']},
      hideLoopback: true, sort: 'connections' as const, locale: 'en'};
    expect(buildAppRows(args).rows[0].destinations[0].acknowledged).toBe(true);
    expect(buildAppRows({...args, extension: {...args.extension, busy: true}}).rows[0].destinations[0].acknowledged).toBe(false);
    expect(buildAppRows({...args, mode: 'demo'}).rows[0].intentCounts.none).toBe(1);
    expect(buildAppRows({...args, connections: []}).rows).toEqual([]);
  });
});

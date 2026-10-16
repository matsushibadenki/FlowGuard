// パス: src/core/identity.ts
import type {Connection, Rule, SigningIdentity} from './types';
export function validIdentity(value: unknown): value is SigningIdentity {
  if (!value || typeof value !== 'object') return false;
  const i = value as SigningIdentity;
  return typeof i.identifier === 'string' && /^[A-Za-z0-9._-]{1,256}$/.test(i.identifier) &&
    ((i.authority === 'apple' && i.teamId === '') ||
     (i.authority === 'developer' && typeof i.teamId === 'string' && /^[A-Z0-9]{10}$/.test(i.teamId)));
}
export function sameIdentity(a: SigningIdentity | null | undefined, b: SigningIdentity | null | undefined) {
  return validIdentity(a) && validIdentity(b) && a.authority === b.authority && a.teamId === b.teamId && a.identifier === b.identifier;
}
export function canSaveRule(c: Connection, mode: 'live' | 'demo') {
  return c.protocol==='TCP' && !!c.appKey && (mode === 'demo' || validIdentity(c.identity));
}
export function legacyRule(r: Rule) { return r.source === 'live' && !r.identity; }
export function identityLabel(i: SigningIdentity) { return `${i.authority === 'apple' ? 'Apple' : i.teamId} / ${i.identifier}`; }

export function identityKey(i: SigningIdentity) { return `signed:v1:${i.authority}:${i.teamId}:${i.identifier}`; }

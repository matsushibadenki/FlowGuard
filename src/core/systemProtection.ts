import type {Connection, Rule} from './types';
import {validIdentity} from './identity';
export function systemPreset(subject: Pick<Connection, 'identity' | 'port' | 'direction'>, protocol:string) {
  if(protocol!=='TCP') return null;
  const identity = subject.identity;
  if (!validIdentity(identity) || identity.authority !== 'apple' || subject.direction === 'inbound') return null;
  if (identity.identifier === 'com.apple.mDNSResponder' && subject.port === 53) return 'systemDNS' as const;
  if (identity.identifier === 'com.apple.softwareupdated' && subject.port === 443) return 'systemUpdates' as const;
  return null;
}
export function systemProtected(rule: Rule) {return rule.source === 'live' && !!systemPreset(rule,'TCP') && !rule.systemException;}

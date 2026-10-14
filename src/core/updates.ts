import type {Snapshot,UpdateSnapshot} from './types';
export function updateBusy(u:UpdateSnapshot){return ['checking','downloading','installing'].includes(u.stage);}
export function installAllowed(s:Pick<Snapshot,'updates'|'extension'|'discovery'|'ports'|'services'>){return s.updates.stage==='ready'&&!s.updates.development&&!s.extension.busy&&!['approval_required','restart_required','configuration_saved'].includes(s.extension.stage)&&[s.discovery.stage,s.ports.stage,s.services.stage].every(stage=>!['running','cancelling'].includes(stage));}
export function updateProgress(u:UpdateSnapshot){return u.total&&u.total>0?Math.min(100,Math.floor(u.downloaded/u.total*100)):null;}

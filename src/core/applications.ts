import type {ApplicationGroup,Connection} from './types';
// Browser preview only. Native groups are projected by Rust from its session.
export function groupApplications(connections:Connection[]):ApplicationGroup[]{
 const groups=new Map<string,ApplicationGroup>();
 for(const c of connections){const id=c.appKey||`unknown:${c.pid}`;let group=groups.get(id);if(!group){group={id,app:c.app,appKey:c.appKey,connectionIds:[],processIds:[],destinations:[]};groups.set(id,group);}group.connectionIds.push(c.id);if(!group.processIds.includes(c.pid))group.processIds.push(c.pid);const destination=`${c.destination}:${c.port}`;if(!group.destinations.includes(destination))group.destinations.push(destination);}
 return [...groups.values()].sort((a,b)=>b.connectionIds.length-a.connectionIds.length||a.id.localeCompare(b.id));
}

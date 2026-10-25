import {DestinationNamesControl} from './DestinationNames';
import {systemPreset} from '../core/systemProtection';
// パス: src/components/Inspector.tsx
import {canSaveRule,identityLabel,sameIdentity} from '../core/identity';
import {ruleApplied} from '../core/enforcement';
import {useState} from 'react';
import {PanelLeft,PanelRight,ExternalLink,ArrowLeftRight} from 'lucide-react';
import type {Action,Connection,Direction,Snapshot} from '../core/types';
import {dispatch,dockPanel,floatPanel,native} from '../core/store';
import {endpoint,ruleFor} from '../core/rules';
import {ActionIcon,AppIcon,DirectionIcon,Notice,type T} from './ui';
export function Inspector({snapshot,connection,t,onError,isFloating=false,mapEditor=false}:{snapshot:Snapshot;connection:Connection|undefined;t:T;onError:(error:unknown)=>void;isFloating?:boolean;mapEditor?:boolean}){
 const existing=connection?ruleFor(snapshot.document.rules,connection,snapshot.document.preferences.mode):undefined;
 const [action,setAction]=useState<Action>(existing?.action??'allow');
 const [direction,setDirection]=useState<Direction>(existing?.direction??(connection?.direction==='unknown'?'both':'outbound'));
 const [busy,setBusy]=useState(false);const [didSave,setDidSave]=useState(false);
 const saved=didSave&&snapshot.document.rules.some(r=>r.source===snapshot.document.preferences.mode&&(snapshot.document.preferences.mode==='demo'?r.appKey===connection?.appKey:sameIdentity(r.identity,connection?.identity))&&r.destination===connection?.destination&&r.port===connection?.port&&r.action===action&&r.direction===direction);
 async function save(){if(!connection)return;setBusy(true);try{await dispatch({type:'saveRule',connectionId:connection.id,action,direction});setDidSave(true);}catch(e){onError(e);}finally{setBusy(false);}}
 return <aside className={`inspector surface ${isFloating?'standalone':''} ${mapEditor?'map-editor':''}`} aria-label={t('details')}><div className="section-head"><h2>{t('details')}</h2><div className="panel-controls"><button className="icon-button" aria-label={t('dockLeft')} onClick={()=>void dockPanel('left').catch(onError)}><PanelLeft size={17}/></button><button className="icon-button" aria-label={t('dockRight')} onClick={()=>void dockPanel('right').catch(onError)}><PanelRight size={17}/></button></div></div>
 {!connection?<div className="empty"><ArrowLeftRight size={28}/><p>{t('empty')}</p></div>:<><div className="inspector-app"><AppIcon app={connection.app} appKey={connection.appKey}/><div><h3>{connection.app}</h3><span>{snapshot.document.preferences.mode==='demo'?t('demo'):connection.pid>0?`PID ${connection.pid}`:'—'}</span><small title={connection.appKey}>{connection.appKey||t('noIdentity')}</small></div></div>
 <DestinationNamesControl snapshot={snapshot} connection={connection}/><dl className="connection-details"><div><dt>{t('destination')}</dt><dd>{endpoint(connection)}</dd></div><div><dt>{t('direction')}</dt><dd><DirectionIcon direction={connection.direction}/>{t(connection.direction==='unknown'?'unknownDirection':connection.direction==='inbound'?'inbound':'outbound')}</dd></div><div><dt>{t('status')}</dt><dd><span className="observed-dot"/>{t('observed')}</dd></div><div><dt>{t('protocol')}</dt><dd>{connection.protocol}</dd></div></dl>
 {snapshot.document.preferences.mode==='live'&&<p className="helper">{connection.identity?identityLabel(connection.identity):t('signatureUnverified')}</p>}<div className="rule-editor"><h3>{t('createRule')}</h3><p>{t('ruleHelp')}</p><fieldset className="action-options"><legend className="sr-only">{t('createRule')}</legend>{(['allow','block','ask'] as Action[]).map(a=><label key={a} className={`action-option ${a} ${action===a?'chosen':''}`}><input type="radio" name="action" value={a} checked={action===a} onChange={()=>{setAction(a);setDidSave(false);}}/><span className="action-symbol"><ActionIcon action={a} size={18}/></span><span><strong>{t(a)}</strong><small>{t(`${a}Help`)}</small></span></label>)}</fieldset>
 <label className="field">{t('direction')}<select value={direction} onChange={e=>{setDirection(e.target.value as Direction);setDidSave(false);}}>{(['outbound','inbound','both'] as Direction[]).map(d=><option key={d} value={d}>{t(d)}</option>)}</select></label>
 {connection.direction==='unknown'&&<p className="helper">{t('saveUnknown')}</p>}<p className="helper">{t('ruleScope')}</p>
 {connection.protocol!=='TCP'&&<p className="helper" role="status">{t('tcpRulesOnly')}</p>}
 {snapshot.document.preferences.mode==='live'&&systemPreset({...connection,direction},connection.protocol)&&<p className="system-protection-warning" role="status">{t(systemPreset({...connection,direction},connection.protocol)!)} · {t('systemProtectedHelp')}</p>}
 {action==='ask'&&<p className="helper">{t('askEditorTimeout').replace('{action}',t(snapshot.document.preferences.askTimeoutAction??'allow'))}</p>}<button className="primary-button" disabled={busy||!canSaveRule(connection,snapshot.document.preferences.mode)} onClick={()=>void save()}>{busy?t('saving'):t(mapEditor?(snapshot.document.preferences.mode==='live'&&snapshot.extension.stage==='applied'?'saveRule':'mapSaveIntent'):'saveIntent')}</button><p className="save-status" aria-live="polite">{existing&&existing.action===action&&existing.direction===direction&&ruleApplied(existing,snapshot.extension)?t('applied'):saved?t('saved'):t('pending')}</p></div>
 <details className="technical-details"><summary>{t('identity')}</summary><dl><dt>{t('local')}</dt><dd>{connection.local}</dd><dt>{t('identity')}</dt><dd>{connection.appKey||t('noIdentity')}</dd><dt>{t('process')}</dt><dd>{connection.pid||'—'}</dd></dl></details></>}
 <Notice snapshot={snapshot} t={t} compact/>{!isFloating&&<button className="float-button" disabled={!native} title={!native?t('browserDemo'):undefined} onClick={()=>void floatPanel().catch(onError)}><ExternalLink size={17}/>{t('float')}</button>}</aside>;
}

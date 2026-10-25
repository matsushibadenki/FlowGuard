import {flowNameText} from './core/flow-names';
import {GpuMap} from './components/GpuMap';
import {invoke} from '@tauri-apps/api/core';
import {PanelDockTargets} from './components/PanelDocking';
import type {GpuFilter} from './core/gpu-panel';
import {HistoryRetention} from './components/HistoryRetention';
import {ProtectionGuide} from './components/ProtectionGuide';
import {AppIconProvider} from './components/AppIcon';
import {EmergencyRecovery} from './components/EmergencyRecovery';
import {AskRequests} from './components/AskRequests';
// パス: src/App.tsx
import {legacyRule} from './core/identity';
import {usePortSheetFocus} from './components/usePortSheetFocus';
import {LanDetails} from './components/LanDetails';
import {useEffect,useMemo,useState} from 'react';
import {Search,Eye,Info,X,ExternalLink} from 'lucide-react';
import {dispatch,floatPanel,useStartupError,native,useSnapshot} from './core/store';
import {translator} from './core/i18n';
import type {Connection,View} from './core/types';
import {extensionLabel,protectionLabel} from './core/enforcement';
import {ruleFor} from './core/rules';
import {Sidebar} from './components/Sidebar';
import {ProtectionStatus} from './components/ProtectionStatus';
import type {SortKey} from './core/mapModel';
import {FlowMap} from './components/FlowMap';
import {Connections} from './components/Connections';
import {Applications} from './components/Applications';
import {Inspector} from './components/Inspector';
import {RuleList} from './components/RuleList';
import {WorkspaceTools} from './components/WorkspaceTools';
import {Settings} from './components/Settings';
import {Brand} from './components/ui';
const panelType=new URLSearchParams(location.search).get('panel');
const floating=panelType==='inspector';
export function App(){
 const snapshot=useSnapshot();
 return <AppIconProvider mode={snapshot?.document.preferences.mode??'demo'} locale={snapshot?.document.preferences.locale??'ja'}><Workspace/></AppIconProvider>;
}
function Workspace(){
 const [hideLoopback,setHideLoopback]=useState(true);const [sort,setSort]=useState<SortKey>('connections');
 const snapshot=useSnapshot();const startupError=useStartupError();const [search,setSearch]=useState('');const [filter,setFilter]=useState<GpuFilter['intent']>('all');const [error,setError]=useState<string|null>(null);const [updateRequested,setUpdateRequested]=useState(false);
 usePortSheetFocus(snapshot?.portsFocus??0);
 useEffect(()=>{if(snapshot?.gpuDockOperation&&snapshot.gpuFilter){setSearch(snapshot.gpuFilter.search);setFilter(snapshot.gpuFilter.intent);setHideLoopback(snapshot.gpuFilter.hideLoopback);}},[snapshot?.gpuDockOperation]);
 useEffect(()=>{if(updateRequested&&snapshot?.document.preferences.view==='settings'){const panel=document.getElementById('software-update');panel?.scrollIntoView({block:'start',behavior:'auto'});panel?.focus({preventScroll:true});setUpdateRequested(false);}},[updateRequested,snapshot?.document.preferences.view]);
 const locale=snapshot?.document.preferences.locale??'ja';const t=translator(locale);
 useEffect(()=>{document.documentElement.lang=locale;},[locale]);
 const onError=(e:unknown)=>{const value=String(e);setError(value.includes('ask_')?t('askNoLongerPending'):value.includes('unsupported_or_invalid_rules')?t('askUnsupported'):value.includes('edit_conflict')?t('editConflict'):value.includes('invalid_backup')?t('invalidBackup'):value.includes('connection_expired')?t('expired'):value.includes('application_identity_unavailable')?t('signatureUnverified'):value.includes('application_identity_changed')||value.includes('migration_conflict')?t('identityChanged'):t('error'));};
 const connections=useMemo(()=>snapshot?.connections.filter(c=>{
  const names=flowNameText(snapshot,c.id);
  const match=`${c.app} ${c.destination} ${c.appKey}${names?' '+names:''}`.toLowerCase().includes(search.toLowerCase());
  return match&&(filter==='all'||(ruleFor(snapshot.document.rules,c,snapshot.document.preferences.mode)?.action??'none')===filter);
 })??[],[snapshot,search,filter]);
 if(!snapshot)return <main className="loading"><Brand large/><h1>{t('loading')}</h1>{startupError&&<><p>{startupError}</p><button className="primary-button" onClick={()=>location.reload()}>{t('retry')}</button></>}</main>;
 const p=snapshot.document.preferences;const selected=snapshot.connections.find(c=>c.id===snapshot.selectedId);
 function navigate(view:View){setSearch('');setFilter('all');void dispatch({type:'setView',view}).catch(onError);}
 function select(c:Connection){void dispatch({type:'select',id:c.id}).catch(onError);}
 const tools=['network','servers','diagnostics'].includes(p.view);
 const category=p.view==='network'?'networkCategory':p.view==='servers'?'serverCategory':p.view==='diagnostics'?'securityCategory':p.view==='settings'?'commonCategory':'firewallCategory';
 const errorBanner=error?<div className="error-banner" role="alert"><Info size={18}/><span>{error}</span><button className="icon-button" aria-label={t('close')} onClick={()=>setError(null)}><X size={18}/></button></div>:null;
 if(panelType==='gpu')return <main className="floating-workspace">{errorBanner}<EmergencyRecovery snapshot={snapshot} t={t} onError={onError}/><GpuMap snapshot={snapshot} filter={{...(snapshot.gpuFilter??{search:'',intent:'all',hideLoopback:true}),source:p.mode}} standalone/><button className="secondary-button" onClick={()=>void invoke('dock_gpu_panel').catch(onError)}>{locale==='ja'?'閉じて通常マップへ':locale==='en'?'Close and return to map':'关闭并返回地图'}</button></main>;
 if(panelType==='lan-details')return <main className="floating-workspace">{errorBanner}<LanDetails snapshot={snapshot} t={t} native={native} standalone/></main>;
 if(floating)return <main className="floating-workspace">{errorBanner}<EmergencyRecovery snapshot={snapshot} t={t} onError={onError}/><AskRequests snapshot={snapshot} t={t} onError={onError}/><Inspector key={`${selected?.id}-${p.mode}`} snapshot={snapshot} connection={selected} t={t} onError={onError} isFloating mapEditor={p.view==='map'}/></main>;
 return <div className="app-shell"><PanelDockTargets locale={locale} onError={onError}/><Sidebar snapshot={snapshot} t={t} navigate={navigate} onError={onError}/><main className={`workspace ${p.view==='map'?'workspace-map':''}`}>{p.view==='map'?<ProtectionStatus snapshot={snapshot} t={t} native={native} onError={onError}/>:<div className="protection-status" role="status"><Eye size={18} aria-hidden="true"/><strong>{t(protectionLabel(snapshot.extension,p.mode,snapshot.document.rules.some(r=>r.source===p.mode)))}</strong><span className="protection-info" tabIndex={0} title={t('protectionHelp')} aria-label={t('protectionHelp')}><Info size={17}/></span></div>}{snapshot.document.rules.some(legacyRule)&&<div className="error-banner legacy-warning" role="status"><Info size={18}/><span>{t('legacyRulesSummary').replace('{n}',String(snapshot.document.rules.filter(legacyRule).length))}</span><button className="secondary-button" onClick={()=>navigate('rules')}>{t('rules')}</button></div>}<EmergencyRecovery snapshot={snapshot} t={t} onError={onError}/><AskRequests snapshot={snapshot} t={t} onError={onError}/><ProtectionGuide snapshot={snapshot}/><header className="workspace-header"><div><span className="workspace-category">{t(category)}</span><h1>{t(p.view)}</h1><p>{t(`${p.view}Subtitle`)}</p></div></header>
 {['available','ready'].includes(snapshot.updates.stage)&&<div className="update-banner surface" role="status"><span>{locale==='ja'?'ソフトウェア更新':locale==='en'?'Software update':'软件更新'} · {snapshot.updates.version}</span><button className="secondary-button" onClick={()=>{setUpdateRequested(true);navigate('settings');}}>{locale==='ja'?'更新を表示':locale==='en'?'View update':'查看更新'}</button></div>}
 {errorBanner}{!native&&<div className="preview-banner"><Eye size={16}/>{t(tools?(p.view==='network'?'interfacePreview':p.view==='servers'?'serverPlan':'diagnosticPlan'):'browserDemo')}</div>}{!tools&&snapshot.firewall.error&&p.mode==='live'&&<div className="error-banner" role="alert"><Info size={17}/>{t('observerError')}</div>}
 {!tools&&p.view!=='map'&&<section className="overview surface" aria-label={t('status')}><div className="overview-source"><div className="source-label"><span className="observed-dot"/><strong>{t(p.mode==='demo'?'demo':'live')}</strong></div><p>{t(p.mode==='demo'?'demoHelp':'liveHelp')}</p><div className="source-switch"><button aria-pressed={p.mode==='live'} disabled={!native} onClick={()=>void dispatch({type:'setMode',mode:'live'}).catch(onError)}>{t('live')}</button><button aria-pressed={p.mode==='demo'} onClick={()=>void dispatch({type:'setMode',mode:'demo'}).catch(onError)}>{t('demo')}</button></div></div><div className="overview-stat"><span>{t('connections')}</span><strong>{snapshot.connections.length}</strong></div><div className="overview-stat"><span>{t('savedRules')}</span><strong>{snapshot.document.rules.filter(r=>r.source===p.mode).length}</strong></div><div className="overview-enforcement"><span>{t('enforcement')}</span><strong><span className="neutral-dot"/>{t(p.mode==='demo'?'observationOnly':extensionLabel(snapshot.extension.stage))}</strong><p>{t(p.mode==='live'&&snapshot.extension.signed?'appliedPolicyHelp':'observationOnly')}</p></div></section>}
 {tools&&<WorkspaceTools snapshot={snapshot} view={p.view} t={t} native={native}/>}
 {(p.view==='map'||p.view==='apps'||p.view==='history')&&<div className={`connection-toolbar ${p.view==='map'?'map-unified-toolbar':''}`}>
 {p.view==='map'&&<label className="map-sort-control"><span>{t('sortBy')}</span><select value={sort} onChange={e=>setSort(e.target.value as SortKey)}><option value="connections">{t('sortConnections')}</option><option value="name">{t('sortName')}</option><option value="attention">{t('sortAttention')}</option></select></label>}
 {p.view!=='history'&&<label className="filter"><span className="sr-only">{t('filter')}</span><select aria-label={t('filter')} value={filter} onChange={e=>setFilter(e.target.value as GpuFilter['intent'])}>{['all','allow','block','ask','none'].map(a=><option key={a} value={a}>{t(a==='none'?'noRule':a as 'all'|'allow'|'block'|'ask')}</option>)}</select></label>}
 {p.view==='map'&&<label className="map-hide-loopback"><input type="checkbox" checked={hideLoopback} onChange={e=>setHideLoopback(e.target.checked)}/>{t('hideLoopback')}</label>}
 <label className="search-field"><Search size={17}/><input aria-label={t('search')} placeholder={t('search')} value={search} maxLength={512} onChange={e=>setSearch(e.target.value.replace(/\0/g,''))}/>{search&&<button className="icon-button" aria-label={t('clearSearch')} onClick={()=>setSearch('')}><X size={15}/></button>}</label></div>}

 {p.view==='map'&&<div className={`map-workspace panel-${p.panel}`}><div className="map-main"><FlowMap gpuFilter={{search,intent:filter,hideLoopback,source:p.mode,searchNames:snapshot.flowNames?.searchEnabled??false}} hideLoopback={hideLoopback} setHideLoopback={setHideLoopback} sort={sort} snapshot={snapshot} connections={connections} t={t} select={select} clearFilters={()=>{setSearch('');setFilter('all');}}/><Connections connections={snapshot.history.slice(0,4)} t={t} locale={locale} select={select} activeIds={snapshot.connections.map(c=>c.id)} title={t('recent')} viewAll={()=>navigate('history')}/></div>{p.panel==='floating'?<div className="floating-placeholder surface"><ExternalLink size={25}/><p>{t('floating')}</p><button className="secondary-button" onClick={()=>void floatPanel().catch(onError)}>{t('openPanel')}</button></div>:<Inspector key={`${selected?.id}-${p.mode}`} snapshot={snapshot} connection={selected} t={t} onError={onError} mapEditor/>}</div>}
 {p.view==='apps'&&<div className={`map-workspace panel-${p.panel}`}><Applications snapshot={snapshot} connections={connections} t={t} locale={locale} select={select}/><Inspector key={`${selected?.id}-${p.mode}`} snapshot={snapshot} connection={selected} t={t} onError={onError}/></div>}
 {p.view==='history'&&<><section className="surface history-controls"><HistoryRetention snapshot={snapshot} compact/></section><Connections connections={snapshot.history.filter(c=>`${c.app} ${c.destination}`.toLowerCase().includes(search.toLowerCase()))} t={t} locale={locale} activeIds={snapshot.connections.map(c=>c.id)} select={c=>{select(c);navigate('map');}} title={t('history')}/></>}
 {p.view==='rules'&&<RuleList snapshot={snapshot} t={t} onError={onError}/>} {p.view==='settings'&&<Settings snapshot={snapshot} t={t} onError={onError}/>}
 <footer className="workspace-footer"><span><Eye size={14}/>{t(tools?(p.view==='network'?(native?'lanTools':'lanDesktopOnly'):p.view==='servers'?'serviceObservations':'localReadOnly'):p.mode==='live'?extensionLabel(snapshot.extension.stage):'observationOnly')}</span><span>{tools?(p.view==='network'&&snapshot.network.observedAt?`${t('lastObserved')} ${new Intl.DateTimeFormat(locale,{hour:'2-digit',minute:'2-digit',second:'2-digit'}).format(snapshot.network.observedAt*1000)}`:p.view==='servers'?`${snapshot.services.rows.length} ${t('serviceObservations')}`:t('diagnostics')):snapshot.lastObserved&&p.mode==='live'?`${t('lastObserved')} ${new Intl.DateTimeFormat(locale,{hour:'2-digit',minute:'2-digit',second:'2-digit'}).format(snapshot.lastObserved*1000)}`:p.mode==='demo'?t('demo'):t('refreshing')}</span></footer>
 </main></div>;
}

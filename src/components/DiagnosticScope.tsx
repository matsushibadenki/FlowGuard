import {useState} from 'react';
import {ArrowRight, Crosshair, Globe2, Info, Network, RefreshCw, Server} from 'lucide-react';
import {diagnosticAction} from '../core/store';
import type {DeclaredScope, DiagnosticFinding, DiagnosticReport} from '../core/types';
import './DiagnosticScope.css';

const words={
 ja:{path:'公開範囲の確認',declared:'想定公開範囲',source:'観測元',observed:'サービス応答',unrecorded:'未記録',unknown:'未確認',confirmed:'応答を確認',unverified:'応答未確認',openDetail:'診断詳細を開く',pathHelp:'申告とこの観測元からの応答を並べています。LAN内からの応答はインターネット公開を証明せず、未確認は非公開を意味しません。',title:'診断範囲',reported:'利用者の申告',help:'この診断に限る利用者の申告です。観測結果や公開状態の証明ではなく、次の診断には引き継がれません。秘密情報を入力しないでください。',purpose:'用途',environment:'環境',owner:'管理責任者',exposure:'想定公開範囲',save:'記録',reload:'最新を読み直す',conflict:'別の画面で更新されました。入力内容を確認し、最新を読み直してください。',error:'記録できません。入力内容を確認してください。'},
 en:{path:'Exposure review',declared:'Intended exposure',source:'Observation source',observed:'Service responses',unrecorded:'Not recorded',unknown:'Unknown',confirmed:'Responses observed',unverified:'Responses unverified',openDetail:'Open diagnostic details',pathHelp:'Compares the declaration with responses from this observation source. A LAN response does not prove internet exposure; unverified does not mean private.',title:'Assessment scope',reported:'User-reported',help:'A declaration for this report only, not proof of observation or exposure. It is not carried to later reports. Do not enter secrets.',purpose:'Purpose',environment:'Environment',owner:'Responsible person',exposure:'Intended exposure',save:'Record',reload:'Reload latest',conflict:'Updated in another window. Review your draft and reload the latest record.',error:'Could not save. Check the fields.'},
 'zh-CN':{path:'公开范围确认',declared:'预期公开范围',source:'观测来源',observed:'服务响应',unrecorded:'未记录',unknown:'未确认',confirmed:'已确认响应',unverified:'响应未确认',openDetail:'打开诊断详情',pathHelp:'并列显示用户申报与此观测来源的响应。局域网内的响应不能证明已向互联网公开；未确认不代表未公开。',title:'诊断范围',reported:'用户申报',help:'仅适用于本次诊断的用户申报，不代表实测或公开状态证明，也不会继承到后续诊断。请勿输入秘密信息。',purpose:'用途',environment:'环境',owner:'负责人',exposure:'预期公开范围',save:'记录',reload:'重新读取',conflict:'其他窗口已更新。请检查输入并重新读取最新记录。',error:'无法记录，请检查输入。'}
};
const empty:DeclaredScope={revision:0,purpose:'',environment:'',owner:'',exposure:''};
export function DiagnosticScope({report,locale,native,selectedFinding,onSelectFinding,navigationBusy=false}:{report:DiagnosticReport;locale:keyof typeof words;native:boolean;selectedFinding?:string|null;onSelectFinding:(finding:DiagnosticFinding)=>void;navigationBusy?:boolean}){
 const w=words[locale], current=report.declaredScope??empty;
 const [draft,setDraft]=useState<DeclaredScope>(()=>({...current}));
 const [busy,setBusy]=useState(false),[error,setError]=useState(false);
 const conflict=current.revision!==draft.revision;
 const services=report.findings.filter(f=>f.category==='service');
 const confirmed=services.filter(f=>f.certainty==='observed').length, unverified=services.length-confirmed;
 async function save(){setBusy(true);setError(false);try{await diagnosticAction('declared_scope',report.id,null,0,null,JSON.stringify(draft));setDraft({...draft,revision:draft.revision+1});}catch{setError(true);}finally{setBusy(false);}}
 return <section className="diagnostic-scope" aria-label={w.title}>
  <header><div><Crosshair size={18} aria-hidden="true"/><h3>{w.title}</h3><span>{w.reported}</span></div><span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span></header>
  <div className="scope-review-head"><h4>{w.path}</h4><span className="info-tip" tabIndex={0} title={w.pathHelp} aria-label={w.pathHelp}><Info size={15}/></span></div>
  <div className="scope-review" aria-label={w.path}>
   <div className="scope-node"><Globe2 size={17} aria-hidden="true"/><span>{w.declared} · {w.reported}</span><strong>{current.exposure.trim()||w.unrecorded}</strong></div>
   <ArrowRight className="scope-arrow" size={16} aria-hidden="true"/>
   <div className="scope-node"><Network size={17} aria-hidden="true"/><span>{w.source}</span><strong>{report.sourceAddress||w.unknown}</strong><small>{[report.interface,report.family].filter(Boolean).join(' · ')}</small></div>
   <ArrowRight className="scope-arrow" size={16} aria-hidden="true"/>
   <div className="scope-node"><Server size={17} aria-hidden="true"/><span>{w.observed} · {report.target}</span><strong>{w.confirmed} {confirmed}</strong><small>{w.unverified} {unverified}</small><div className="scope-ports">{services.map(f=><button type="button" key={f.id} data-observed={f.certainty==='observed'} aria-pressed={selectedFinding===f.id} disabled={!native||navigationBusy} aria-label={`${w.openDetail}: ${f.port} · ${f.certainty==='observed'?w.confirmed:w.unverified} · ${new Date(f.observedAt*1000).toLocaleString(locale)}`} title={`${w.openDetail} · ${new Date(f.observedAt*1000).toLocaleString(locale)}`} onClick={()=>onSelectFinding(f)}>{f.port}</button>)}</div></div>
  </div>
  <div className="diagnostic-scope-form">{(['purpose','environment','owner','exposure'] as const).map(field=><label key={field}><span>{w[field]}</span><input maxLength={64} value={draft[field]} onChange={e=>setDraft({...draft,[field]:e.target.value})}/></label>)}<button disabled={!native||busy||conflict} onClick={()=>void save()}>{w.save}</button></div>
  {conflict&&<div className="diagnostic-scope-conflict" role="status"><span>{w.conflict}</span><button onClick={()=>{setDraft({...current});setError(false);}}><RefreshCw size={14} aria-hidden="true"/>{w.reload}</button></div>}{error&&<p role="alert">{w.error}</p>}
 </section>;
}

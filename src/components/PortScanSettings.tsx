import {Gauge,Leaf,Hourglass,SlidersHorizontal,Info,Layers,Timer,Pause,Hash} from 'lucide-react';
import {defaultPortSettings,estimatePortBudget,portBreakdown,portJobBudgetMs,portSettingsWords,portSpeedPresets,type PortScanSettings as Settings} from '../core/port-settings';
import type {Locale,PortSnapshot} from '../core/types';
import './PortScanSettings.css';
const fields=[{key:'concurrency',label:'concurrency',Icon:Layers,min:1,max:8,step:1},{key:'timeoutMs',label:'timeout',Icon:Timer,min:100,max:3000,step:100},{key:'intervalMs',label:'interval',Icon:Pause,min:0,max:1000,step:50},{key:'maxPorts',label:'maxPorts',Icon:Hash,min:1,max:256,step:1}] as const;
export function PortScanSettings({value,change,count,disabled,locale}:{value:Settings;change:(value:Settings)=>void;count:number;disabled:boolean;locale:Locale}){
 const w=portSettingsWords[locale],estimate=estimatePortBudget(value,count);
 return <div className="port-scan-config"><div className="port-speed-presets" role="group" aria-label={w.draft}>{([{key:'quiet',Icon:Leaf},{key:'balanced',Icon:Gauge},{key:'patient',Icon:Hourglass}] as const).map(({key,Icon})=><button key={key} disabled={disabled} aria-pressed={Object.entries(portSpeedPresets[key]).every(([k,v])=>value[k as keyof Settings]===v)} onClick={()=>change({...portSpeedPresets[key]})}><Icon size={17}/><span>{w[key]}</span></button>)}<span className="info-tip" tabIndex={0} title={w.help} aria-label={w.help}><Info size={16}/></span></div>
 <details><summary><SlidersHorizontal size={16}/>{w.title}</summary><div className="port-scan-fields">{fields.map(({key,label,Icon,min,max,step})=><label key={key}><span><Icon size={15}/>{w[label]}{(key==='timeoutMs'||key==='intervalMs')&&<small>{w.ms}</small>}</span><input aria-label={`${w[label]}${key==='timeoutMs'||key==='intervalMs'?' (ms)':''}`} type="number" min={min} max={max} step={step} disabled={disabled} value={Number.isNaN(value[key])?'':value[key]} onChange={e=>change({...value,[key]:e.target.valueAsNumber})}/></label>)}</div></details>
 <div className="port-budget" data-invalid={estimate===null||estimate>portJobBudgetMs}><Timer size={15}/><span>{w.estimate}</span><strong>{estimate===null?'—':`${(estimate/1000).toLocaleString(locale,{maximumFractionDigits:1})}s`}</strong><span>/ {portJobBudgetMs/1000}s</span><span>· {count} TCP</span></div>
 {count>0&&(estimate===null||estimate>portJobBudgetMs)&&<p role="status" className="port-budget-error">{w.invalid}</p>}
 </div>;
}
export function PortScanVisual({job,locale}:{job:PortSnapshot;locale:Locale}){
 const w=portSettingsWords[locale],counts=portBreakdown(job),settings=job.settings??defaultPortSettings;
 let offset=0;const slices=(['open','closed','uncertain','pending'] as const).map(key=>{const length=counts[key]/Math.max(1,counts.total)*100,start=offset;offset+=length;return {key,length,start};});
 const label=slices.map(s=>`${w[s.key]}: ${counts[s.key]}`).join(' · ');
 return <div className="port-scan-visual"><div className="port-outcome-ring" role="img" aria-label={label}><svg viewBox="0 0 100 100" aria-hidden="true"><circle className="port-ring-base" cx="50" cy="50" r="42"/>{slices.filter(s=>s.length>0).map(s=><circle key={s.key} data-kind={s.key} pathLength="100" cx="50" cy="50" r="42" strokeDasharray={`${s.length} ${100-s.length}`} strokeDashoffset={-s.start}/>)}</svg><div><strong>{job.completed}</strong><small>/ {job.total}</small></div></div>
 <div className="port-outcome-counts">{slices.map(({key})=><div key={key} data-kind={key}><i/><strong>{counts[key]}</strong><span>{w[key]}</span></div>)}</div>
 <div className="port-captured-settings" aria-label={w.captured}><small className="port-captured-label">{w.captured}</small><span title={w.concurrency}><Layers size={15}/>{settings.concurrency}</span><span title={w.timeout}><Timer size={15}/>{settings.timeoutMs}ms</span><span title={w.interval}><Pause size={15}/>{settings.intervalMs}ms</span><span title={w.maxPorts}><Hash size={15}/>{settings.maxPorts}</span><span className="info-tip" tabIndex={0} title={`${w.chartHelp} ${w.help}`} aria-label={`${w.chartHelp} ${w.help}`}><Info size={16}/></span></div>
 </div>;
}

import {invoke,isTauri} from '@tauri-apps/api/core';
export interface AppIconAsset{dataUrl:string;width:number;height:number;source:'macos_workspace'}
const MAX=128,TTL=60_000,NEGATIVE_TTL=15_000;
const cache=new Map<string,{at:number;asset:AppIconAsset|null}>();
const pending=new Map<string,Promise<AppIconAsset|null>>();
const queue:(()=>void)[]=[];let active=0;
export function validAppIcon(value:unknown):value is AppIconAsset{
 if(!value||typeof value!=='object')return false;const v=value as AppIconAsset;
 if(v.source!=='macos_workspace'||!Number.isInteger(v.width)||v.width<1||v.width>96||!Number.isInteger(v.height)||v.height<1||v.height>96||typeof v.dataUrl!=='string'||v.dataUrl.length>65558||!v.dataUrl.startsWith('data:image/png;base64,'))return false;
 const data=v.dataUrl.slice(22);if(!/^[A-Za-z0-9+/]+={0,2}$/.test(data))return false;
 try{const header=atob(data.slice(0,44));const b=(i:number)=>header.charCodeAt(i);const size=(i:number)=>(b(i)*2**24+b(i+1)*2**16+b(i+2)*256+b(i+3));return header.startsWith('\x89PNG\r\n\x1a\n')&&header.slice(12,16)==='IHDR'&&size(16)===v.width&&size(20)===v.height;}catch{return false;}
}
function drain(){while(active<4&&queue.length){active++;queue.shift()!();}}
/** Bounded presentation cache; authoritative path admission and raster generation stay in Rust. */
export function loadAppIcon(subject:string):Promise<AppIconAsset|null>{
 if(!isTauri()||!subject||subject.length>4096)return Promise.resolve(null);
 const found=cache.get(subject);if(found&&performance.now()-found.at<(found.asset?TTL:NEGATIVE_TTL)){cache.delete(subject);cache.set(subject,found);return Promise.resolve(found.asset);}
 const current=pending.get(subject);if(current)return current;
 if(pending.size>=128)return Promise.resolve(null);
 const queued=performance.now();
 const work=new Promise<AppIconAsset|null>(resolve=>{
  queue.push(()=>{if(performance.now()-queued>4000){pending.delete(subject);active--;resolve(null);drain();return;}void invoke<unknown>('application_icon',{subject}).then(value=>validAppIcon(value)?value:null).catch(()=>null).then(asset=>{
   cache.delete(subject);cache.set(subject,{at:performance.now(),asset});while(cache.size>MAX)cache.delete(cache.keys().next().value!);resolve(asset);
  }).finally(()=>{pending.delete(subject);active--;drain();});});
 });pending.set(subject,work);drain();return work;
}
export const appIconWords={ja:{native:'macOSのアプリアイコン。署名・本人性・保護状態の証明ではありません。',fallback:'アイコン未取得・汎用表示'},en:{native:'macOS application icon. Not proof of signing, identity or protection.',fallback:'Icon unavailable · generic display'},'zh-CN':{native:'macOS应用图标，不代表签名、身份或防护已验证。',fallback:'未获取图标・通用显示'}};

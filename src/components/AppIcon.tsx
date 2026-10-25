import {createContext,useContext,useEffect,useMemo,useRef,useState,type ReactNode} from 'react';
import {AppWindow} from 'lucide-react';
import type {Locale} from '../core/types';
import {appIconWords,loadAppIcon} from '../core/app-icons';
const Context=createContext<{mode:'live'|'demo';locale:Locale}>({mode:'demo',locale:'ja'});
export function AppIconProvider({mode,locale,children}:{mode:'live'|'demo';locale:Locale;children:ReactNode}){const value=useMemo(()=>({mode,locale}),[mode,locale]);return <Context.Provider value={value}>{children}</Context.Provider>;}
export function AppIcon({app,appKey,small=false,demo=false}:{app:string;appKey?:string;small?:boolean;demo?:boolean}){
 const {mode,locale}=useContext(Context),w=appIconWords[locale],root=useRef<HTMLSpanElement>(null);
 const [visible,setVisible]=useState(false),[image,setImage]=useState<{subject:string;url:string}|null>(null),[broken,setBroken]=useState<{subject:string;url:string}|null>(null);
 useEffect(()=>{const node=root.current;if(!node)return;if(typeof IntersectionObserver==='undefined'){setVisible(true);return;}const observer=new IntersectionObserver(entries=>{setVisible(entries.some(e=>e.isIntersecting));},{rootMargin:'80px'});observer.observe(node);return()=>observer.disconnect();},[appKey]);
 const subject=appKey??'',enabled=mode==='live'&&!demo&&visible&&!!subject;
 useEffect(()=>{if(!enabled){setImage(null);return;}let alive=true,timer=0;async function refresh(){const asset=await loadAppIcon(subject);if(!alive)return;setImage(asset?{subject,url:asset.dataUrl}:null);timer=window.setTimeout(()=>void refresh(),asset?60_000:15_000);}void refresh();return()=>{alive=false;clearTimeout(timer);};},[enabled,subject]);
 const url=enabled&&image?.subject===subject&&!(broken?.subject===subject&&broken.url===image.url)?image?.url:null;
 return <span ref={root} className={`app-icon ${url?'native':'generic'} ${small?'small':''}`} title={`${app} · ${url?w.native:w.fallback}`} aria-hidden="true">{url?<img src={url} width={small?32:42} height={small?32:42} alt="" draggable={false} decoding="async" onError={()=>setBroken({subject,url})}/>:<AppWindow size={small?17:25} strokeWidth={1.7}/>}</span>;
}

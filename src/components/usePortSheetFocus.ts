import {useEffect} from 'react';
// Presentation only: Rust owns the navigation request and selected target.
export function usePortSheetFocus(request:number){
 useEffect(()=>{if(request>0){const sheet=document.getElementById('lan-tcp-sheet');sheet?.scrollIntoView({block:'start',behavior:'auto'});sheet?.focus({preventScroll:true});}},[request]);
}

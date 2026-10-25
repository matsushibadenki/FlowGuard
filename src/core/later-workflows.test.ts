import {describe,it,expect} from 'vitest';
import {acceptsPanel,panelDragType,panelDragValue,dockingWords} from './panel-docking';
import {evidenceRows} from './evidence-sheet';
import {diagnosticWords} from './diagnostics';
describe('evidence workspaces and panel docking',()=>{
 it('requires the exact panel drag type and value',()=>{expect(acceptsPanel([panelDragType],panelDragValue)).toBe(true);expect(acceptsPanel(['text/plain'],panelDragValue)).toBe(false);expect(acceptsPanel([panelDragType],'arbitrary-panel')).toBe(false);});
 it('filters, sorts and preserves original evidence',()=>{const rows=[{id:2,status:'unknown',text:'MAIL'},{id:1,status:'observed',text:'mail'}];expect(evidenceRows(rows,' MAIL ','all',r=>r.text,r=>r.status,(a,b)=>a.id-b.id).map(r=>r.id)).toEqual([1,2]);expect(evidenceRows(rows,'','observed',r=>r.text,r=>r.status,()=>0).map(r=>r.id)).toEqual([1]);expect(rows[0].id).toBe(2);});
 it('has complete three-language docking and diagnosis labels',()=>{for(const dictionary of [dockingWords,diagnosticWords])for(const words of Object.values(dictionary))expect(Object.keys(words).sort()).toEqual(Object.keys(dictionary.en).sort());});
});

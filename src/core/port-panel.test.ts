import {describe,it,expect} from 'vitest';
import {portPanelWords,portPanelStages} from './port-panel';
describe('TCP detail panels',()=>{
 it('provides every dock destination and lifecycle message in three languages',()=>{
  const keys=Object.keys(portPanelWords.ja);for(const locale of ['en','zh-CN'] as const){expect(Object.keys(portPanelWords[locale])).toEqual(keys);expect(Object.values(portPanelWords[locale]).every(Boolean)).toBe(true);expect(Object.keys(portPanelStages[locale])).toEqual(Object.keys(portPanelStages.ja));expect(Object.values(portPanelStages[locale]).every(Boolean)).toBe(true);}
 });
});

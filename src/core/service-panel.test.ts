import {describe,it,expect} from 'vitest';
import {servicePanelWords} from './service-panel';
import {serviceWords} from './service-i18n';
describe('service detail panel labels',()=>{
 it('includes every panel and read-only note boundary in three languages',()=>{for(const locale of ['en','zh-CN'] as const){expect(Object.keys(servicePanelWords[locale])).toEqual(Object.keys(servicePanelWords.ja));expect(Object.values(servicePanelWords[locale]).every(Boolean)).toBe(true);expect(Object.keys(serviceWords[locale])).toEqual(Object.keys(serviceWords.ja));}});
});

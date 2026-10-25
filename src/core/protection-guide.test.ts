import {describe,it,expect} from 'vitest';
import {needsProtectionGuide,protectionGuideWords} from './protection-guide';
describe('protection status guide',()=>{
 it('requires explicit acknowledgement including legacy preferences',()=>{expect(needsProtectionGuide({})).toBe(true);expect(needsProtectionGuide({protectionGuideSeen:false})).toBe(true);expect(needsProtectionGuide({protectionGuideSeen:true})).toBe(false);});
 it('provides four distinct states and matching controls in all three languages',()=>{const keys=Object.keys(protectionGuideWords.en).sort();for(const words of Object.values(protectionGuideWords)){expect(Object.keys(words).sort()).toEqual(keys);expect(words.steps).toHaveLength(4);expect(new Set(words.steps.map(s=>s.title)).size).toBe(4);expect(words.steps.every(s=>s.title&&s.text)).toBe(true);}});
});

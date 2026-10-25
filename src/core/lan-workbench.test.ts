import {describe,it,expect} from 'vitest';
import {lanColumns,validLanColumns,lanColumnLabels} from './lan-workbench';
import {columnWord} from './lan-sheet';
import type {Locale} from './types';
import {dictionaries} from './i18n';
describe('LAN column contract',()=>{
 it('requires address and detail controls, rejects duplicates/unknowns',()=>{
  expect(validLanColumns([...lanColumns])).toBe(true);
  expect(validLanColumns(['address','inspect'])).toBe(true);
  for(const columns of [[],['address'],['inspect'],['address','inspect','address'],['address','inspect','bogus']])expect(validLanColumns(columns)).toBe(false);
 });
 it('keeps every field translated in all supported locales',()=>{
  for(const column of lanColumns)for(const [locale,dictionary] of Object.entries(dictionaries))expect((columnWord(column,locale as Locale)??dictionary[lanColumnLabels[column]]).length).toBeGreaterThan(0);
 });
});

import {describe,expect,it} from 'vitest';
import {exposureCandidate} from './exposure-candidates';

describe('port-based exposure candidates',()=>{
 it('labels common management, database and sharing ports as candidates',()=>{
  expect([22,3389,3306,6379,2375,445].map(exposureCandidate)).toEqual(['SSH','RDP','MySQL','Redis','Docker API','SMB']);
 });
 it('does not invent a service for an unfamiliar port',()=>{
  expect(exposureCandidate(51857)).toBeNull();
 });
});

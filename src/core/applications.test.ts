import {describe,it,expect} from 'vitest';
import {groupApplications} from './applications';
import type {Connection} from './types';
const connection:Connection={id:'one',app:'Browser',appKey:'/Applications/Browser.app/Contents/MacOS/Browser',pid:10,local:'127.0.0.1',destination:'example.com',port:443,protocol:'TCP',direction:'unknown',observedAt:1};
describe('application aggregation',()=>{
 it('combines connections from the same executable without losing selectable IDs',()=>{
  const groups=groupApplications([connection,{...connection,id:'two',pid:11},{...connection,id:'three',destination:'other.example',port:80}]);
  expect(groups).toHaveLength(1);expect(groups[0].connectionIds).toEqual(['one','two','three']);expect(groups[0].processIds).toEqual([10,11]);expect(groups[0].destinations).toHaveLength(2);
 });
 it('does not merge different executable paths or unidentified processes with the same name',()=>{
  expect(groupApplications([connection,{...connection,id:'other',appKey:'/tmp/Browser'},{...connection,id:'unknown',appKey:'',pid:20},{...connection,id:'unknown2',appKey:'',pid:21}])).toHaveLength(4);
 });
});

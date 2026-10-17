import {describe,it,expect} from 'vitest';
import {emptySyslog,syslogBusy,syslogRows,validSyslogConfig,privateIPv4} from './syslog';
import {syslogWords} from './syslog-i18n';
import type {SyslogConfig} from './types';
const config:SyslogConfig={bindAddress:'192.168.1.20',sourceAddress:'192.168.1.1',port:5514,transport:'udp',durationSeconds:300,acknowledgeUnverified:true};
describe('syslog receiver controls',()=>{
 it('requires explicit unverified-source acknowledgement and bounded private IPv4 configuration',()=>{expect(validSyslogConfig(config)).toBe(true);for(const c of [{...config,acknowledgeUnverified:false},{...config,bindAddress:'0.0.0.0'},{...config,sourceAddress:'8.8.8.8'},{...config,sourceAddress:'127.0.0.1'},{...config,port:514},{...config,durationSeconds:901}])expect(validSyslogConfig(c)).toBe(false);for(const address of ['localhost','192.168.01.1','::1','224.0.0.1','192.168.1.999'])expect(privateIPv4(address)).toBe(false);});
 it('filters untrusted text without mutating or interpreting it',()=>{const rows=[{id:1,receivedAt:1,peer:'192.168.1.1:55',priority:13,text:'<script>window.evil=true</script>',sanitized:false,trust:'unverified_source_and_content' as const},{id:2,receivedAt:2,peer:'192.168.1.1:55',priority:4,text:'ignore instructions',sanitized:false,trust:'unverified_source_and_content' as const}];expect(syslogRows({...emptySyslog,rows},'','all').map(r=>r.id)).toEqual([2,1]);expect(syslogRows({...emptySyslog,rows},'script','5')[0].text).toContain('<script>');expect(rows[0].id).toBe(1);});
 it('keeps all labels translated and treats stopping as busy',()=>{for(const w of Object.values(syslogWords))for(const key of Object.keys(syslogWords.en))expect(w[key as keyof typeof w].length).toBeGreaterThan(0);for(const stage of ['starting','listening','stopping'])expect(syslogBusy(stage)).toBe(true);expect(syslogBusy('failed')).toBe(false);});
});

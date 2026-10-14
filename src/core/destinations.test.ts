// パス: src/core/destinations.test.ts
import {describe, expect, it} from 'vitest';
import {classifyAddress} from './destinations';
describe('address ranges without hostname or route inference', () => {
  it.each([
    ['127.0.0.1', 'loopback'], ['127.255.255.255', 'loopback'], ['::1', 'loopback'], ['[::1]', 'loopback'],
    ['192.168.1.12', 'local'], ['172.15.0.1', 'public'], ['172.16.0.1', 'local'], ['172.32.0.1', 'public'],
    ['100.63.255.255', 'public'], ['100.64.0.1', 'local'], ['100.127.255.255', 'local'], ['100.128.0.1', 'public'],
    ['169.254.1.1', 'local'], ['fe80::1%en0', 'local'], ['febf::1', 'local'], ['fec0::1', 'other'],
    ['fd00::1', 'local'], ['::ffff:10.0.0.1', 'local'], ['::ffff:7f00:1', 'loopback'],
    ['2001:db8::1', 'public'], ['150.171.110.134', 'public'], ['224.0.0.251', 'other'],
    ['0.0.0.0', 'other'], ['240.1.2.3', 'other'], ['::', 'other'], ['ff02::1', 'other'],
    ['not-an-ip', 'other'], ['1.2.3.999', 'other'], ['2001:::1', 'other'], ['::1::2', 'other'],
    ['[::1]:443', 'other'], ['fe80::1%', 'other'], ['1:2:3:4:5:6:7:8:9', 'other'],
  ])('%s is %s', (address, expected) => expect(classifyAddress(address)).toBe(expected));
});

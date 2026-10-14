// パス: src/core/destinations.ts
export type DestinationClass = 'loopback' | 'local' | 'public' | 'other';
function ipv4(address: string): number[] | null {
  if (!/^\d{1,3}(\.\d{1,3}){3}$/.test(address)) return null;
  const parts = address.split('.').map(Number);
  return parts.every(p => p <= 255) ? parts : null;
}
function classifyV4(p: number[]): DestinationClass {
  const [a, b] = p;
  if (a === 127) return 'loopback';
  if (a === 10 || (a === 172 && b >= 16 && b <= 31) ||
      (a === 192 && b === 168) || (a === 169 && b === 254) ||
      (a === 100 && b >= 64 && b <= 127)) return 'local';
  return a === 0 || a >= 224 ? 'other' : 'public';
}
export function classifyAddress(address: string): DestinationClass {
  let value = address;
  if (value.startsWith('[') && value.endsWith(']')) value = value.slice(1, -1);
  const zone = value.indexOf('%');
  if (zone >= 0) {
    if (!value.includes(':') || !/^[\w.-]+$/.test(value.slice(zone + 1))) return 'other';
    value = value.slice(0, zone);
  }
  const v4 = ipv4(value);
  if (v4) return classifyV4(v4);
  if (!value.includes(':')) return 'other';
  if (value.includes('.')) {
    const lastColon = value.lastIndexOf(':');
    const embedded = ipv4(value.slice(lastColon + 1));
    if (!embedded) return 'other';
    value = value.slice(0, lastColon + 1) +
      ((embedded[0] << 8) | embedded[1]).toString(16) + ':' +
      ((embedded[2] << 8) | embedded[3]).toString(16);
  }
  const halves = value.split('::');
  if (halves.length > 2) return 'other';
  const left = halves[0] ? halves[0].split(':') : [];
  const right = halves.length === 2 && halves[1] ? halves[1].split(':') : [];
  const pieces = [...left, ...right];
  if (!pieces.every(p => /^[\da-f]{1,4}$/i.test(p))) return 'other';
  if (halves.length === 1 ? pieces.length !== 8 : pieces.length >= 8) return 'other';
  const words = [...left.map(p => parseInt(p, 16)),
    ...Array(8 - pieces.length).fill(0), ...right.map(p => parseInt(p, 16))];
  if (words.slice(0, 5).every(p => p === 0) && words[5] === 0xffff) {
    return classifyV4([words[6] >> 8, words[6] & 255, words[7] >> 8, words[7] & 255]);
  }
  if (words.slice(0, 7).every(p => p === 0) && words[7] === 1) return 'loopback';
  const first = words[0];
  if ((first & 0xffc0) === 0xfe80 || (first & 0xfe00) === 0xfc00) return 'local';
  return (first & 0xe000) === 0x2000 ? 'public' : 'other';
}

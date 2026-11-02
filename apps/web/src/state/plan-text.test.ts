import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { TRIMMED, trimText } from './plan-text';

describe('trimText', () => {
  it('trims the core set at both ends, U+FEFF and U+0085 alike', () => {
    expect(trimText('﻿ porch \u0085')).toBe('porch');
    expect(trimText('　 a b\t\n')).toBe('a b');
    expect(trimText('')).toBe('');
    expect(trimText(' ﻿ ')).toBe('');
  });

  it('keeps what neither side trims', () => {
    expect(trimText('​x​')).toBe('​x​');
    expect(trimText('a\u0085b')).toBe('a\u0085b');
  });

  it('holds the same characters as the core', async () => {
    const { readFileSync } = await import('node:fs');
    const core = readFileSync(
      resolve(import.meta.dirname, '../../../../crates/maquette-model/src/plans/text.rs'),
      'utf8'
    );
    const list = core.slice(core.indexOf('TRIMMED'), core.indexOf('];'));
    const chars = [...list.matchAll(/'\\u\{([0-9A-Fa-f]+)\}'/g)].map(m =>
      String.fromCodePoint(parseInt(m[1] ?? '', 16))
    );
    expect(chars).toHaveLength(26);
    expect(new Set(TRIMMED)).toEqual(new Set(chars));
  });
});

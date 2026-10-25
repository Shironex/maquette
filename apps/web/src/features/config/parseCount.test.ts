import { describe, expect, it } from 'vitest';
import { parseCount } from './parseCount';

describe('parseCount', () => {
  it('reads plain, grouped and k counts', () => {
    expect(parseCount('50000')).toBe(50_000);
    expect(parseCount(' 50,000 ')).toBe(50_000);
    expect(parseCount('50k')).toBe(50_000);
    expect(parseCount('1.5K')).toBe(1500);
  });

  it('refuses anything else', () => {
    expect(parseCount('')).toBeNull();
    expect(parseCount('fifty')).toBeNull();
    expect(parseCount('-3')).toBeNull();
  });
});

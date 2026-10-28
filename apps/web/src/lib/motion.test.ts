import { afterEach, describe, expect, it, vi } from 'vitest';
import { flightStart, reducedMotion } from './motion';

function prefers(reduce: boolean) {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: reduce && query === '(prefers-reduced-motion: reduce)',
  }));
}

afterEach(() => vi.unstubAllGlobals());

describe('reduced motion', () => {
  it('follows the system setting', () => {
    prefers(true);
    expect(reducedMotion()).toBe(true);
    prefers(false);
    expect(reducedMotion()).toBe(false);
  });

  it('starts a flight a whole flight ago when motion is reduced, so it lands at once', () => {
    vi.spyOn(performance, 'now').mockReturnValue(5000);
    prefers(false);
    expect(flightStart(1100)).toBe(5000);
    prefers(true);
    expect(flightStart(1100)).toBe(3900);
  });
});

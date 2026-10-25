import { describe, expect, it } from 'vitest';
import {
  FADE,
  HOLD,
  LAST_STOP,
  across,
  clampProgress,
  nearestStop,
  presence,
  progressAt,
  scrollFor,
  smoothstep,
} from './progress';

describe('progressAt', () => {
  const top = 100;
  const span = 6000;

  it('counts stops from the top of the tour to the end of its scroll', () => {
    expect(progressAt(top, top, span)).toBe(0);
    expect(progressAt(top + span / 2, top, span)).toBe(3);
    expect(progressAt(top + span, top, span)).toBe(LAST_STOP);
    expect(progressAt(top + span / 4, top, span)).toBeCloseTo(1.5);
  });

  it('clamps above and below the tour', () => {
    expect(progressAt(0, top, span)).toBe(0);
    expect(progressAt(top + span * 2, top, span)).toBe(LAST_STOP);
  });

  it('stays at 0 when the tour does not scroll', () => {
    expect(progressAt(500, top, 0)).toBe(0);
    expect(progressAt(500, top, -40)).toBe(0);
  });

  it('round trips with scrollFor', () => {
    for (const stop of [0, 1, 2.5, 6]) {
      expect(progressAt(scrollFor(stop, top, span), top, span)).toBeCloseTo(stop);
    }
    expect(scrollFor(9, top, span)).toBe(top + span);
  });
});

describe('clampProgress and nearestStop', () => {
  it('ignores values that are not numbers', () => {
    expect(clampProgress(Number.NaN)).toBe(0);
    expect(clampProgress(Number.POSITIVE_INFINITY)).toBe(0);
  });

  it('names the nearest stop, switching halfway between two', () => {
    expect(nearestStop(0.49)).toBe(0);
    expect(nearestStop(0.5)).toBe(1);
    expect(nearestStop(2.7)).toBe(3);
    expect(nearestStop(-3)).toBe(0);
    expect(nearestStop(42)).toBe(LAST_STOP);
  });
});

describe('smoothstep', () => {
  it('eases from 0 to 1 between its edges', () => {
    expect(smoothstep(-1, 0, 1)).toBe(0);
    expect(smoothstep(0.5, 0, 1)).toBe(0.5);
    expect(smoothstep(2, 0, 1)).toBe(1);
    expect(smoothstep(0.1, 0, 1)).toBeLessThan(0.1);
  });
});

describe('across', () => {
  it('runs from 0 to 1 between its edges and clamps outside', () => {
    expect(across(3.7, 3.7, 4.3)).toBe(0);
    expect(across(4, 3.7, 4.3)).toBeCloseTo(0.5);
    expect(across(5, 3.7, 4.3)).toBe(1);
    expect(across(1, 2, 2)).toBe(0);
    expect(across(2, 2, 2)).toBe(1);
  });
});

describe('presence', () => {
  it('is full while the tour holds on the stop and gone by FADE', () => {
    expect(presence(4, 4)).toBe(1);
    expect(presence(4 + HOLD, 4)).toBe(1);
    expect(presence(4 - FADE, 4)).toBe(0);
    expect(presence(3.5, 4)).toBe(0);
    const mid = presence(4 + (HOLD + FADE) / 2, 4);
    expect(mid).toBeGreaterThan(0);
    expect(mid).toBeLessThan(1);
  });

  it('cuts on the nearest stop with reduced motion', () => {
    expect(presence(4.3, 4, true)).toBe(1);
    expect(presence(4.5, 4, true)).toBe(0);
    expect(presence(4.5, 5, true)).toBe(1);
  });
});

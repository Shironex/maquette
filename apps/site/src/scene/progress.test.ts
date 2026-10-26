import { describe, expect, it } from 'vitest';
import {
  FADE,
  HOLD,
  LAST_STOP,
  across,
  clampProgress,
  nearestStop,
  presence,
  progressFromEdges,
  scrollForProgress,
  smoothstep,
} from './progress';

describe('progressFromEdges', () => {
  // Seven 900 px sections from the top of the page: the hero, then six stops.
  const edges = Array.from({ length: 8 }, (_, i) => i * 900);

  it('reaches a stop as its section meets the top of the window', () => {
    expect(progressFromEdges(0, edges)).toBe(0);
    expect(progressFromEdges(900, edges)).toBe(1);
    expect(progressFromEdges(1350, edges)).toBeCloseTo(1.5);
    expect(progressFromEdges(5400, edges)).toBe(LAST_STOP);
  });

  it('clamps above the tour and past the last stop', () => {
    expect(progressFromEdges(-200, edges)).toBe(0);
    expect(progressFromEdges(6000, edges)).toBe(LAST_STOP);
    expect(progressFromEdges(99_999, edges)).toBe(LAST_STOP);
  });

  it('follows sections of uneven height', () => {
    const uneven = [100, 1100, 1600, 2600];
    expect(progressFromEdges(100, uneven)).toBe(0);
    expect(progressFromEdges(1350, uneven)).toBeCloseTo(1.5);
    expect(progressFromEdges(1600, uneven)).toBe(2);
  });

  it('stays at 0 without enough sections', () => {
    expect(progressFromEdges(500, [])).toBe(0);
    expect(progressFromEdges(500, [0, 900])).toBe(0);
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

describe('scrollForProgress', () => {
  const edges = Array.from({ length: 8 }, (_, i) => i * 900);

  it('is the inverse of progressFromEdges', () => {
    for (const progress of [0, 0.5, 1, 2.25, 5.9, LAST_STOP]) {
      expect(progressFromEdges(scrollForProgress(progress, edges), edges)).toBeCloseTo(progress);
    }
    const uneven = [100, 1100, 1600, 2600];
    expect(scrollForProgress(1.5, uneven)).toBe(1350);
  });

  it('clamps outside the tour', () => {
    expect(scrollForProgress(-1, edges)).toBe(0);
    expect(scrollForProgress(99, edges)).toBe(scrollForProgress(LAST_STOP, edges));
    expect(scrollForProgress(1, [])).toBe(0);
  });
});

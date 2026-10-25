import { describe, expect, it } from 'vitest';
import { fitFov } from './fit-fov';

describe('fitting a stop to the view', () => {
  it('keeps the stop field of view in a view at least as wide as 16:9', () => {
    expect(fitFov(40, 16 / 9)).toBe(40);
    expect(fitFov(40, 2.4)).toBe(40);
    expect(fitFov(40, 0)).toBe(40);
  });

  it('widens it in a narrower view so the whole frame shows', () => {
    const wider = fitFov(40, 1);
    expect(wider).toBeGreaterThan(40);
    const half = (n: number) => Math.tan((n * Math.PI) / 360);
    expect(half(wider)).toBeCloseTo((half(40) * 16) / 9);
  });
});

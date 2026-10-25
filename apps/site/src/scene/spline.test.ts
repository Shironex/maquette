import { describe, expect, it } from 'vitest';
import { createMonotone } from './spline';

describe('createMonotone', () => {
  const spline = createMonotone([0, 1, 2, 4], [0, 1, 1, 3]);

  it('passes through every key and clamps outside them', () => {
    expect(spline.at(0)).toBe(0);
    expect(spline.at(1)).toBeCloseTo(1);
    expect(spline.at(4)).toBeCloseTo(3);
    expect(spline.at(-5)).toBe(0);
    expect(spline.at(9)).toBe(3);
  });

  it('holds perfectly still across a run of equal keys', () => {
    for (const x of [1, 1.25, 1.5, 1.75, 2]) expect(spline.at(x)).toBeCloseTo(1, 9);
  });

  it('never steps back or overshoots between rising keys', () => {
    // A slow stretch next to a fast one, like a stop's move beside a flight.
    const timing = createMonotone([0, 0.4, 1, 1.4], [0, 0.1, 2, 2.1]);
    let previous = timing.at(0);
    for (let x = 0.01; x <= 1.4; x += 0.01) {
      const value = timing.at(x);
      expect(value).toBeGreaterThanOrEqual(previous - 1e-12);
      expect(value).toBeLessThanOrEqual(2.1 + 1e-12);
      previous = value;
    }
  });

  it('keeps moving through a key between two moving stretches', () => {
    const timing = createMonotone([0, 0.4, 1], [0, 0.1, 2]);
    expect(timing.slope(0.4)).toBeGreaterThan(0);
    expect(timing.slope(0.7)).toBeGreaterThan(timing.slope(0.4));
  });

  it('has a slope that matches its values', () => {
    const x = 0.83;
    const numeric = (spline.at(x + 1e-5) - spline.at(x - 1e-5)) / 2e-5;
    expect(spline.slope(x)).toBeCloseTo(numeric, 4);
  });
});

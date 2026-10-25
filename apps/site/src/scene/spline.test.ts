import { describe, expect, it } from 'vitest';
import { createSpline } from './spline';

const out: number[] = [];

describe('createSpline', () => {
  const spline = createSpline([0, 1, 2, 4], [[0], [1], [1], [3]]);

  it('passes through every key and clamps outside them', () => {
    expect(spline.at(0, out)).toEqual([0]);
    expect(spline.at(1, out)[0]).toBeCloseTo(1);
    expect(spline.at(4, out)[0]).toBeCloseTo(3);
    expect(spline.at(-5, out)).toEqual([0]);
    expect(spline.at(9, out)[0]).toBeCloseTo(3);
  });

  it('holds perfectly still across a run of equal keys', () => {
    for (const x of [1, 1.25, 1.5, 1.75, 2]) expect(spline.at(x, out)[0]).toBeCloseTo(1, 9);
  });

  it('starts and ends at rest', () => {
    const e = 1e-4;
    expect(spline.at(e, out)[0]).toBeLessThan(1e-3);
    const end = spline.at(4 - e, out)[0] ?? 0;
    expect(3 - end).toBeLessThan(1e-3);
  });

  it('keeps its speed through a key between moving segments', () => {
    const line = createSpline([0, 1, 3, 4], [[0], [1], [3], [4]]);
    const slope = (x: number) =>
      ((line.at(x + 1e-4, out)[0] ?? 0) - (line.at(x - 1e-4, out)[0] ?? 0)) / 2e-4;
    expect(slope(1)).toBeCloseTo(slope(3));
    expect(slope(1 - 1e-3)).toBeCloseTo(slope(1 + 1e-3), 2);
  });

  it('interpolates every component', () => {
    const pair = createSpline(
      [0, 1],
      [
        [0, 10],
        [2, 20],
      ]
    );
    expect(pair.at(0.5, out)).toEqual([1, 15]);
  });
});

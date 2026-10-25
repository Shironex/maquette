/**
 * A monotone cubic through keyed values (Fritsch and Carlson): smooth, with
 * no overshoot, so a value that only grows between keys never steps back and
 * a run of equal keys holds still. The camera path uses it to time the
 * travel along its curves. No three.js imports.
 */

export interface Spline {
  /** The value at `x`, clamped to the keys. */
  at(x: number): number;
  /** The slope at `x` (0 outside the keys). */
  slope(x: number): number;
}

/** `keys` must rise strictly; `values` has one value per key. */
export function createMonotone(keys: readonly number[], values: readonly number[]): Spline {
  const count = Math.min(keys.length, values.length);
  const key = (i: number) => keys[i] ?? 0;
  const value = (i: number) => values[i] ?? 0;
  const secants = Array.from({ length: Math.max(0, count - 1) }, (_, i) => {
    const span = key(i + 1) - key(i);
    return span > 0 ? (value(i + 1) - value(i)) / span : 0;
  });
  const tangents = Array.from({ length: count }, (_, i) => {
    const before = secants[i - 1];
    const after = secants[i];
    if (before === undefined) return after ?? 0;
    if (after === undefined) return before;
    // Opposite signs or a flat side: an extremum or a hold, so it rests here.
    if (before * after <= 0) return 0;
    // The weighted harmonic mean keeps the curve inside each segment's range.
    const a = key(i) - key(i - 1);
    const b = key(i + 1) - key(i);
    return (3 * (a + b)) / ((2 * b + a) / before + (b + 2 * a) / after);
  });

  const segment = (x: number) => {
    let i = 0;
    while (i < count - 2 && x > key(i + 1)) i++;
    return i;
  };

  return {
    at(x) {
      if (count === 0) return 0;
      if (count === 1 || x <= key(0)) return value(0);
      if (x >= key(count - 1)) return value(count - 1);
      const i = segment(x);
      const span = key(i + 1) - key(i);
      const t = (x - key(i)) / span;
      const t2 = t * t;
      const t3 = t2 * t;
      return (
        (2 * t3 - 3 * t2 + 1) * value(i) +
        (t3 - 2 * t2 + t) * span * (tangents[i] ?? 0) +
        (-2 * t3 + 3 * t2) * value(i + 1) +
        (t3 - t2) * span * (tangents[i + 1] ?? 0)
      );
    },
    slope(x) {
      if (count < 2 || x < key(0) || x > key(count - 1)) return 0;
      const i = segment(x);
      const span = key(i + 1) - key(i);
      const t = (x - key(i)) / span;
      const t2 = t * t;
      return (
        ((6 * t2 - 6 * t) * value(i)) / span +
        (3 * t2 - 4 * t + 1) * (tangents[i] ?? 0) +
        ((-6 * t2 + 6 * t) * value(i + 1)) / span +
        (3 * t2 - 2 * t) * (tangents[i + 1] ?? 0)
      );
    },
  };
}

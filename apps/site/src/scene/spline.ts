/**
 * A cubic spline through keyed values (any number of components) with
 * Catmull-Rom tangents measured in key space, so uneven key spacing keeps
 * its speed. A key next to an identical one gets a flat tangent, so a run
 * of equal keys holds perfectly still and the motion eases in and out of
 * it; both ends start and stop at rest. No three.js imports.
 */

export interface Spline {
  /** The value at `x`, clamped to the keys, written into `out`. */
  at(x: number, out: number[]): number[];
}

const SAME = 1e-9;

function gap(a: readonly number[], b: readonly number[]): number {
  return a.reduce((sum, v, i) => sum + (v - (b[i] ?? 0)) ** 2, 0);
}

export function createSpline(
  keys: readonly number[],
  values: readonly (readonly number[])[]
): Spline {
  const count = Math.min(keys.length, values.length);
  const width = values[0]?.length ?? 0;
  const tangents = values.slice(0, count).map((value, i) => {
    const before = values[i - 1];
    const after = values[i + 1];
    const flat = new Array<number>(width).fill(0);
    if (!before || !after || gap(before, value) < SAME || gap(value, after) < SAME) return flat;
    const span = (keys[i + 1] ?? 0) - (keys[i - 1] ?? 0);
    return value.map((_, c) => ((after[c] ?? 0) - (before[c] ?? 0)) / span);
  });

  return {
    at(x, out) {
      out.length = width;
      if (count === 0) return out;
      let i = 0;
      while (i < count - 2 && x > (keys[i + 1] ?? 0)) i++;
      const from = keys[i] ?? 0;
      const span = count > 1 ? (keys[i + 1] ?? from) - from : 0;
      const t = span > 0 ? Math.min(1, Math.max(0, (x - from) / span)) : 0;
      const t2 = t * t;
      const t3 = t2 * t;
      const h00 = 2 * t3 - 3 * t2 + 1;
      const h10 = t3 - 2 * t2 + t;
      const h01 = -2 * t3 + 3 * t2;
      const h11 = t3 - t2;
      const a = values[i] ?? [];
      const b = values[i + 1] ?? a;
      const ma = tangents[i] ?? [];
      const mb = tangents[i + 1] ?? ma;
      for (let c = 0; c < width; c++) {
        out[c] =
          h00 * (a[c] ?? 0) +
          h10 * span * (ma[c] ?? 0) +
          h01 * (b[c] ?? 0) +
          h11 * span * (mb[c] ?? 0);
      }
      return out;
    },
  };
}

/**
 * A smooth curve through a list of points, walked by distance. The camera
 * path has one for the camera and one for what it looks at: centripetal
 * Catmull-Rom, which does not loop or overshoot where points bunch up (a
 * slow move at a stop next to a long flight between stops).
 */
import * as THREE from 'three';

/** Samples per segment for the distance table. */
const SAMPLES = 48;

export interface Track {
  /** Distance along the curve at each of its points, from the first. */
  readonly lengths: readonly number[];
  /** The point `distance` along the curve, clamped to its ends, written into `out`. */
  pointAt(distance: number, out: THREE.Vector3): THREE.Vector3;
}

export function createTrack(points: readonly THREE.Vector3[]): Track {
  const curve = new THREE.CatmullRomCurve3([...points], false, 'centripetal');
  const segments = Math.max(1, points.length - 1);
  const total = segments * SAMPLES;
  const table = [0];
  const previous = curve.getPoint(0);
  const next = new THREE.Vector3();
  for (let j = 1; j <= total; j++) {
    curve.getPoint(j / total, next);
    table.push((table[j - 1] ?? 0) + next.distanceTo(previous));
    previous.copy(next);
  }
  const lengths = points.map((_, k) => table[k * SAMPLES] ?? 0);

  return {
    lengths,
    pointAt(distance, out) {
      if (points.length < 2) return out.copy(points[0] ?? out.set(0, 0, 0));
      // The last sample at or before `distance`, by bisection.
      let low = 0;
      let high = total;
      while (high - low > 1) {
        const mid = (low + high) >> 1;
        if ((table[mid] ?? 0) <= distance) low = mid;
        else high = mid;
      }
      const from = table[low] ?? 0;
      const span = (table[high] ?? from) - from;
      const f = span > 0 ? Math.min(1, Math.max(0, (distance - from) / span)) : 0;
      return curve.getPoint((low + f) / total, out);
    },
  };
}

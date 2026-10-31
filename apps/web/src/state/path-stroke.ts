/**
 * Draw on surfaces: what a drag over the models leaves on a path. A drag
 * samples far more points than a path wants, so a stroke is thinned to the
 * ones that hold its shape and to what the path still has room for.
 * Pure data in, data out.
 */
import type { Vec3 } from './inspector';
import { MAX_POINTS, addPoint, snapPoint, type PathsDraft } from './paths';

/** A sample this close to the one before it, in metres, adds nothing. */
export const STROKE_SPACING = 0.02;
/** How far a kept line may stray from the drag, in metres. */
export const STROKE_TOLERANCE = 0.03;

const dist = (a: Vec3, b: Vec3) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);

/** How far `p` is from the segment `a` to `b`. */
function offLine(p: Vec3, a: Vec3, b: Vec3): number {
  const ab: Vec3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  const len = ab[0] ** 2 + ab[1] ** 2 + ab[2] ** 2;
  if (len === 0) return dist(p, a);
  const t = ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / len;
  const k = Math.min(1, Math.max(0, t));
  return dist(p, [a[0] + ab[0] * k, a[1] + ab[1] * k, a[2] + ab[2] * k]);
}

/** Ramer-Douglas-Peucker: the fewest samples that stay within `tolerance` of the drag. */
export function thinStroke(samples: readonly Vec3[], tolerance = STROKE_TOLERANCE): Vec3[] {
  if (samples.length < 3) return [...samples];
  const keep = new Set([0, samples.length - 1]);
  const stack: [number, number][] = [[0, samples.length - 1]];
  while (stack.length > 0) {
    const [from, to] = stack.pop()!;
    let far = -1;
    let most = tolerance;
    for (let i = from + 1; i < to; i += 1) {
      const off = offLine(samples[i]!, samples[from]!, samples[to]!);
      if (off > most) {
        most = off;
        far = i;
      }
    }
    if (far < 0) continue;
    keep.add(far);
    stack.push([from, far], [far, to]);
  }
  return samples.filter((_, i) => keep.has(i));
}

/** At most `room` of the points, evenly picked, always with the last when two or more fit. */
export function fitStroke(points: readonly Vec3[], room: number): Vec3[] {
  if (points.length <= room) return [...points];
  if (room <= 0) return [];
  if (room === 1) return [points[0]!];
  const step = (points.length - 1) / (room - 1);
  return Array.from({ length: room }, (_, i) => points[Math.round(i * step)]!);
}

/**
 * Adds a stroke after `after` (the end when null), thinned and snapped.
 * Returns the draft and the index of its last point, -1 when it added none.
 */
export function addStroke(
  draft: PathsDraft,
  id: string,
  stroke: readonly Vec3[],
  after: number | null,
  snap: boolean
): { draft: PathsDraft; index: number } {
  const path = draft.paths.find(p => p.id === id);
  if (!path) return { draft, index: -1 };
  const points = fitStroke(thinStroke(stroke), MAX_POINTS - path.points.length);
  let next = draft;
  let index = -1;
  let at = after;
  for (const point of points) {
    const added = addPoint(next, id, snapPoint(point, snap), at);
    if (added.index < 0) break;
    next = added.draft;
    index = added.index;
    at = index;
  }
  return { draft: next, index };
}

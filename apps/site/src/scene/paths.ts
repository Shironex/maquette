/**
 * Maquette's paths: curves through the scene, drawn as tubes, that light up
 * from their first point as the page scrolls. Pure data and maths; no DOM
 * and no three.js (./path-mesh.ts builds them).
 *
 * layout.json carries them as Maquette writes them (metres, Y up):
 *
 *   "paths": [{ "id": "main-feed", "name": "Main feed", "curve": "catmull-rom",
 *               "points": [[0.2, 0, 1.1], [0.4, 0, 0.6], [0.4, 0.8, 0.2]],
 *               "start": "trench", "end": "panel", "width": 0.035,
 *               "color": "#f2c230", "glow": "#ffe08a" }],
 *   "timeline": { "paths": { "main-feed": { "easing": "linear",
 *     "keys": [{ "stop": "sakura", "amount": 0 }, { "stop": "canopy", "amount": 1 }] } } }
 *
 * The amount is eased between keyed stops like an explode track. Maquette
 * moves an anchored end with its model when it saves, so the points are
 * where the path is; `start` and `end` are only a record of what it links.
 */
import type { Vec3 } from './manifest';
import { keyedAt, type PathTrack } from './timeline';

export interface ScenePath {
  id: string;
  name: string;
  curve: 'polyline' | 'catmull-rom';
  points: Vec3[];
  start: string | null;
  end: string | null;
  width: number;
  color: string;
  glow: string;
}

/** The share of a path's length the lit front fades over. */
export const SOFT = 0.04;
/** How wide the bright head at the lit front is, as a share of the length. */
export const HEAD = 0.06;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** How much of the path is lit at tour progress `progress`, 0 to 1; 0 without a track. */
export function energizeAt(
  track: PathTrack | undefined,
  stopIds: readonly string[],
  progress: number
): number {
  return track ? keyedAt(track, stopIds, progress) : 0;
}

/**
 * How lit the path is at `u` (0 at the first point, 1 at the last, by
 * length) when `amount` of it is energized: 1 behind the front, fading to
 * 0 over SOFT ahead of it. The front starts just before the path and ends
 * just past it, so 0 is dark everywhere and 1 lit everywhere.
 */
export function litAt(u: number, amount: number): number {
  const front = clamp(amount, 0, 1) * (1 + SOFT);
  return clamp((front - u) / SOFT, 0, 1);
}

/** The bright head riding the lit front while the path fills, 0 once it is dark or full. */
export function headAt(u: number, amount: number): number {
  const a = clamp(amount, 0, 1);
  if (a <= 0 || a >= 1) return 0;
  const front = a * (1 + SOFT) - SOFT;
  return clamp(1 - Math.abs(u - front) / HEAD, 0, 1);
}

/** The path's length through its points, straight runs between them. */
export function polylineLength(points: readonly Vec3[]): number {
  let total = 0;
  for (let i = 1; i < points.length; i += 1) {
    const a = points[i - 1]!;
    const b = points[i]!;
    total += Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2]);
  }
  return total;
}

/**
 * How many rings the tube has along its length: about one per two widths,
 * enough for a curve to read as smooth, 16 to 256.
 */
export function tubeSegments(path: Pick<ScenePath, 'points' | 'width'>): number {
  const step = Math.max(path.width * 2, 0.01);
  return clamp(Math.ceil(polylineLength(path.points) / step), 16, 256);
}

/** The sides of each ring. */
export const TUBE_SIDES = 8;

/** The paths with every Y raised by `lift`, as the site lifts the layout onto the bench. */
export function liftPaths(paths: readonly ScenePath[] | undefined, lift: number): ScenePath[] {
  return (paths ?? [])
    .filter(p => p.points.length >= 2)
    .map(p => ({ ...p, points: p.points.map(([x, y, z]): Vec3 => [x, y + lift, z]) }));
}

/**
 * Where a prop's click and hover triggers sit on the page: its placement
 * seen through the camera between the two stops either side of the
 * playhead. The tour's own camera curves between stops, so a pin between
 * stops is close, and exact at a stop. Pure data in, data out.
 */
import type { SceneLayout } from '@/ipc/client';
import { vec3, type Vec3 } from './inspector';

export interface Hotspot {
  key: string;
  kind: 'click' | 'hover';
  /** "Click · Breakers on". */
  label: string;
  /** Share of the page's window, 0 at the left and top. */
  x: number;
  y: number;
}

/** The aspect the stops are framed for; a narrower window widens the view, as the site does. */
const DESIGN_ASPECT = 16 / 10;
/** Pins of one prop stack this far apart, as a share of the window's height. */
const STACK = 0.055;

const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const unit = (a: Vec3): Vec3 => {
  const len = Math.hypot(a[0], a[1], a[2]) || 1;
  return [a[0] / len, a[1] / len, a[2] / len];
};
const mix = (a: Vec3, b: Vec3, t: number): Vec3 => [
  a[0] + (b[0] - a[0]) * t,
  a[1] + (b[1] - a[1]) * t,
  a[2] + (b[2] - a[2]) * t,
];

export interface PageCamera {
  position: Vec3;
  target: Vec3;
  fov: number;
}

/** The camera at fractional stop `at`, or null with no stops. */
export function cameraAt(stops: SceneLayout['stops'], at: number): PageCamera | null {
  if (stops.length === 0) return null;
  const clamped = Math.min(stops.length - 1, Math.max(0, at));
  const low = stops[Math.floor(clamped)]!;
  const high = stops[Math.ceil(clamped)]!;
  const t = clamped - Math.floor(clamped);
  return {
    position: mix(vec3(low.position), vec3(high.position), t),
    target: mix(vec3(low.target), vec3(high.target), t),
    fov: (low.fov ?? 35) + ((high.fov ?? 35) - (low.fov ?? 35)) * t,
  };
}

/** `point` as shares of a window of `aspect`, or null behind the camera or outside the window. */
export function project(
  camera: PageCamera,
  point: Vec3,
  aspect: number
): { x: number; y: number } | null {
  const forward = unit(sub(camera.target, camera.position));
  const right = unit(cross(forward, [0, 1, 0]));
  const up = cross(right, forward);
  const off = sub(point, camera.position);
  const depth = dot(off, forward);
  if (depth <= 0.01) return null;
  const half = Math.tan((camera.fov * Math.PI) / 360);
  const tall = aspect >= DESIGN_ASPECT ? half : (half * DESIGN_ASPECT) / aspect;
  const x = 0.5 + dot(off, right) / (depth * tall * aspect) / 2;
  const y = 0.5 - dot(off, up) / (depth * tall) / 2;
  return x < 0 || x > 1 || y < 0 || y > 1 ? null : { x, y };
}

/** Every click and hover trigger of a placed prop the camera sees at stop `at`. */
export function hotspotsAt(layout: SceneLayout, at: number, aspect: number): Hotspot[] {
  const camera = cameraAt(layout.stops, at);
  if (!camera) return [];
  return Object.entries(layout.states).flatMap(([asset, object]) => {
    const placement = layout.models[asset];
    if (!object || !placement) return [];
    const spot = project(camera, vec3(placement.position), aspect);
    if (!spot) return [];
    const live = object.triggers.filter(t => t.kind === 'click' || t.kind === 'hover');
    return live.map((t, i) => {
      const kind = t.kind === 'click' ? ('click' as const) : ('hover' as const);
      const state = object.states.find(s => s.id === t.to)?.name ?? t.to;
      return {
        key: `${asset}-${i}`,
        kind,
        label: `${kind === 'click' ? 'Click' : 'Hover'} · ${state}`,
        x: spot.x,
        y: Math.max(0, spot.y - i * STACK),
      };
    });
  });
}

/** How many click and hover triggers the layout's placed props have, seen or not. */
export function hotspotCount(layout: SceneLayout): number {
  return Object.entries(layout.states).reduce((sum, [asset, o]) => {
    if (!layout.models[asset]) return sum;
    return sum + (o?.triggers.filter(t => t.kind === 'click' || t.kind === 'hover').length ?? 0);
  }, 0);
}

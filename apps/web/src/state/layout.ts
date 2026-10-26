/**
 * The Layout editor's logic: the layout with plain numbers, where each
 * model stands, its size, the camera stops and the undo history. Plain
 * functions, so the tests need no renderer.
 */
import type { FirstView, LayoutModel, LayoutView, SceneLayout } from '@/ipc/client';
import { formatBytes } from '@/lib/format';
import { num, vec3, type CameraPose, type Vec3 } from './inspector';

export type { CameraPose };

export interface PlainPlacement {
  /** Metres, y up. */
  position: Vec3;
  /** Degrees, applied X, then Y, then Z. */
  rotation: Vec3;
  scale: number;
}

export interface PlainStop {
  id: string;
  name: string;
  position: Vec3;
  target: Vec3;
  fov: number;
  /** The models it counts for the first view, set by hand; null measures the view. */
  models: string[] | null;
}

export interface PlainLayout {
  models: Record<string, PlainPlacement>;
  stops: PlainStop[];
  /** Width and depth in metres, or no desk. */
  desk: [number, number] | null;
}

export type Tool = 'translate' | 'rotate' | 'scale';

export const LAYOUT_VERSION = 1;
export const DEFAULT_FOV = 35;
export const MAX_STOPS = 24;
export const STOP_NAME_MAX = 60;
export const DEFAULT_DESK: [number, number] = [1.6, 0.9];
export const DESK_THICKNESS = 0.04;
/** The snap steps: 5 cm, 15 degrees, 0.05 of the size. */
export const SNAP = { move: 0.05, rotate: 15, scale: 0.05 } as const;
export const MIN_SCALE = 0.01;
export const MAX_SCALE = 100;
/** Undo steps kept. */
export const HISTORY = 60;

export function fromLayout(layout: SceneLayout): PlainLayout {
  const models: Record<string, PlainPlacement> = {};
  for (const [id, p] of Object.entries(layout.models)) {
    models[id] = { position: vec3(p.position), rotation: vec3(p.rotation), scale: num(p.scale) };
  }
  return {
    models,
    stops: layout.stops.map(s => ({
      id: s.id,
      name: s.name,
      position: vec3(s.position),
      target: vec3(s.target),
      fov: s.fov ?? DEFAULT_FOV,
      models: s.models,
    })),
    desk: layout.desk ? [num(layout.desk.size[0]), num(layout.desk.size[1])] : null,
  };
}

/**
 * The layout the editor saves. Object states and clip choices belong to the
 * Motion view and the timeline to the Timeline view; the app keeps the saved
 * ones when the editor saves.
 */
export function toLayout(layout: PlainLayout): SceneLayout {
  return {
    version: LAYOUT_VERSION,
    models: layout.models,
    stops: layout.stops,
    desk: layout.desk ? { size: layout.desk } : null,
    states: {},
    clips: {},
    timeline: { explode: {}, paths: {} },
    paths: [],
  };
}

/** Where a model stands: its placement, else its side by side spot. */
export function placementOf(layout: PlainLayout, model: LayoutModel): PlainPlacement {
  return (
    layout.models[model.asset] ?? {
      position: vec3(model.fallback),
      rotation: [0, 0, 0],
      scale: 1,
    }
  );
}

/**
 * The layout with every model pinned where it stands now, so a model added
 * later is the only one that moves when the build lines models up.
 */
export function pinAll(layout: PlainLayout, models: LayoutModel[]): PlainLayout {
  const pinned = { ...layout.models };
  for (const model of models) pinned[model.asset] ??= placementOf(layout, model);
  return { ...layout, models: pinned };
}

export function sameLayout(a: PlainLayout, b: PlainLayout): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** The model's own bounds, or a 10 cm cube when they could not be read. */
function boundsOf(model: LayoutModel): { min: Vec3; max: Vec3 } {
  if (!model.min || !model.max) return { min: [-0.05, 0, -0.05], max: [0.05, 0.1, 0.05] };
  return { min: vec3(model.min), max: vec3(model.max) };
}

/** Width, height and depth in centimetres at the placement's scale. */
export function sizeCm(model: LayoutModel, scale: number): Vec3 {
  const { min, max } = boundsOf(model);
  return [0, 1, 2].map(i => (max[i]! - min[i]!) * scale * 100) as Vec3;
}

const rad = (deg: number) => (deg * Math.PI) / 180;

/** `v` rotated as three.js does for an XYZ Euler: Rx · Ry · Rz · v. */
export function rotate(v: Vec3, rotation: Vec3): Vec3 {
  let [x, y, z] = v;
  const [rx, ry, rz] = rotation.map(rad) as Vec3;
  [x, y] = [x * Math.cos(rz) - y * Math.sin(rz), x * Math.sin(rz) + y * Math.cos(rz)];
  [x, z] = [x * Math.cos(ry) + z * Math.sin(ry), -x * Math.sin(ry) + z * Math.cos(ry)];
  [y, z] = [y * Math.cos(rx) - z * Math.sin(rx), y * Math.sin(rx) + z * Math.cos(rx)];
  return [x, y, z];
}

/** The model's bounds in the scene, after its placement. */
export function placedBounds(model: LayoutModel, p: PlainPlacement): { min: Vec3; max: Vec3 } {
  const { min, max } = boundsOf(model);
  const lo: Vec3 = [Infinity, Infinity, Infinity];
  const hi: Vec3 = [-Infinity, -Infinity, -Infinity];
  for (const x of [min[0], max[0]]) {
    for (const y of [min[1], max[1]]) {
      for (const z of [min[2], max[2]]) {
        const r = rotate([x * p.scale, y * p.scale, z * p.scale], p.rotation);
        for (let i = 0; i < 3; i++) {
          const v = r[i]! + p.position[i]!;
          lo[i] = Math.min(lo[i]!, v);
          hi[i] = Math.max(hi[i]!, v);
        }
      }
    }
  }
  return { min: lo, max: hi };
}

/** The placement moved up or down so the model's lowest point rests on y = 0. */
export function dropToFloor(model: LayoutModel, p: PlainPlacement): PlainPlacement {
  const { min } = placedBounds(model, p);
  return { ...p, position: [p.position[0], round(p.position[1] - min[1], 4), p.position[2]] };
}

export function round(n: number, digits = 4): number {
  const f = 10 ** digits;
  return Math.round(n * f) / f;
}

/** Rounds a placement the way it is saved: tenth of a millimetre, hundredth of a degree. */
export function tidy(p: PlainPlacement): PlainPlacement {
  return {
    position: p.position.map(v => round(v, 4)) as Vec3,
    rotation: p.rotation.map(v => round(normalDeg(v), 2)) as Vec3,
    scale: round(Math.min(MAX_SCALE, Math.max(MIN_SCALE, p.scale)), 4),
  };
}

/** Degrees in (-180, 180]. */
export function normalDeg(deg: number): number {
  const d = ((((deg + 180) % 360) + 360) % 360) - 180;
  return d === -180 ? 180 : d;
}

/** "Moon island" to "moon-island", as the app names ids. */
export function slugify(name: string): string {
  return name
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** A free stop id for `name`: its slug, else the slug with `-2`, `-3`. */
export function stopId(stops: PlainStop[], name: string): string {
  const base = slugify(name) || 'stop';
  const taken = (id: string) => stops.some(s => s.id === id);
  if (!taken(base)) return base;
  for (let n = 2; ; n++) if (!taken(`${base}-${n}`)) return `${base}-${n}`;
}

/** "Stop 3", the first number free. */
export function nextStopName(stops: PlainStop[]): string {
  for (let n = stops.length + 1; ; n++) {
    const name = `Stop ${n}`;
    if (!stops.some(s => s.name === name)) return name;
  }
}

export function addStop(layout: PlainLayout, pose: CameraPose): PlainLayout {
  const name = nextStopName(layout.stops);
  const stop: PlainStop = {
    id: stopId(layout.stops, name),
    name,
    position: pose.position.map(v => round(v)) as Vec3,
    target: pose.target.map(v => round(v)) as Vec3,
    fov: round(pose.fov, 2),
    models: null,
  };
  return { ...layout, stops: [...layout.stops, stop] };
}

export function updateStop(
  layout: PlainLayout,
  id: string,
  patch: Partial<Omit<PlainStop, 'id'>>
): PlainLayout {
  return { ...layout, stops: layout.stops.map(s => (s.id === id ? { ...s, ...patch } : s)) };
}

export function removeStop(layout: PlainLayout, id: string): PlainLayout {
  return { ...layout, stops: layout.stops.filter(s => s.id !== id) };
}

/** Moves a stop one place earlier (-1) or later (1) in the tour. */
export function moveStop(layout: PlainLayout, id: string, by: -1 | 1): PlainLayout {
  const from = layout.stops.findIndex(s => s.id === id);
  const to = from + by;
  if (from < 0 || to < 0 || to >= layout.stops.length) return layout;
  const stops = [...layout.stops];
  const [stop] = stops.splice(from, 1);
  stops.splice(to, 0, stop!);
  return { ...layout, stops };
}

/** A problem that keeps the layout from saving, in words, or null. */
export function layoutProblem(layout: PlainLayout): string | null {
  if (layout.stops.length > MAX_STOPS) return `Keep to ${MAX_STOPS} camera stops or fewer.`;
  for (const stop of layout.stops) {
    if (!stop.name.trim()) return 'Give every camera stop a name.';
    if (stop.name.trim().length > STOP_NAME_MAX)
      return `Keep stop names to ${STOP_NAME_MAX} characters or fewer.`;
    if (stop.position.every((v, i) => Math.abs(v - stop.target[i]!) < 1e-4))
      return `${stop.name} looks at its own position. Move its target.`;
  }
  return null;
}

export interface History {
  past: PlainLayout[];
  future: PlainLayout[];
}

/** Records `before` as an undo step. */
export function record(history: History, before: PlainLayout): History {
  return { past: [...history.past, before].slice(-HISTORY), future: [] };
}

export function undo(history: History, now: PlainLayout): [History, PlainLayout] | null {
  const before = history.past[history.past.length - 1];
  if (!before) return null;
  return [{ past: history.past.slice(0, -1), future: [now, ...history.future] }, before];
}

export function redo(history: History, now: PlainLayout): [History, PlainLayout] | null {
  const next = history.future[0];
  if (!next) return null;
  return [{ past: [...history.past, now], future: history.future.slice(1) }, next];
}

/** The first view in megabytes against its target. */
export function firstViewMb(view: FirstView | null | undefined): { value: number; target: number } {
  return { value: num(view?.bytes) / 1e6, target: view?.targetMb ?? 3 };
}

/** One sentence on what the first stop loads, for a tooltip. */
export function firstViewLine(view: FirstView | null | undefined): string {
  if (!view) return 'The first view is measured once the layout loads.';
  if (!view.stop) return 'Add a camera stop in the Inspector layout to measure the first view.';
  const { value, target } = firstViewMb(view);
  const count = view.models.length;
  const models = count === 1 ? '1 model' : `${count} models`;
  const how = view.custom ? 'picked by hand' : 'in its frame';
  return `First view (${view.stop}): ${models} ${how}, ${formatBytes(value * 1e6)} of ${target} MB.`;
}

/** The asset ids the first stop counts, for the model list. */
export function firstViewIds(view: LayoutView | null): Set<string> {
  return new Set(view?.firstView.models.map(m => m.asset) ?? []);
}

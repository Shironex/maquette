/**
 * The Paths editor's rules: curves through the scene, each a list of points
 * in the layout's space (metres, y up) the site draws as a tube and lights
 * from its first point as the page scrolls. An end anchored to a model
 * follows that model's placement. Pure data in, data out.
 */
import type {
  LayoutModel,
  LayoutStop,
  Placement,
  PathTrack,
  ScenePath,
  SceneLayout,
} from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { formatBytes } from '@/lib/format';
import { num, vec3, type Vec3 } from './inspector';
import { SNAP, fromLayout, placementOf, round, slugify } from './layout';

/** What the editor edits: every path and their energize tracks by path id. */
export interface PathsDraft {
  paths: ScenePath[];
  tracks: Record<string, PathTrack>;
}

export const EMPTY_PATHS: PathsDraft = { paths: [], tracks: {} };

/** The path being edited and, when one is picked, its point. */
export type PathSelection = { path: string; point: number | null } | null;

/** Select picks and drags points; Add points puts one where the viewport is clicked. */
export type PathTool = 'select' | 'add';

export type PathEnd = 'start' | 'end';

export const MAX_PATHS = 32;
export const MAX_POINTS = 128;
export const PATH_NAME_MAX = 60;
/** In metres, as the app checks them. */
export const MIN_WIDTH = 0.001;
export const MAX_WIDTH = 1;
export const DEFAULT_WIDTH = 0.035;
export const DEFAULT_COLOR = '#f2c230';
export const DEFAULT_GLOW = '#ffe08a';
/** The tube's sides and its rings per length, as the site builds it (apps/site path-mesh.ts). */
export const TUBE_SIDES = 8;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function draftOf(layout: SceneLayout): PathsDraft {
  const tracks: Record<string, PathTrack> = {};
  for (const [id, track] of Object.entries(layout.timeline.paths)) {
    if (track) tracks[id] = structuredClone(track);
  }
  return { paths: structuredClone(layout.paths), tracks };
}

export function samePaths(a: PathsDraft, b: PathsDraft): boolean {
  return deepEqual(a, b);
}

export const pointsOf = (path: ScenePath): Vec3[] => path.points.map(p => vec3(p));

/** A free path id for `name`: its slug, else the slug with `-2`, `-3`. */
export function pathId(paths: readonly ScenePath[], name: string): string {
  const base = slugify(name) || 'path';
  const taken = (id: string) => paths.some(p => p.id === id);
  if (!taken(base)) return base;
  for (let n = 2; ; n++) if (!taken(`${base}-${n}`)) return `${base}-${n}`;
}

/** "Path 3", the first number free. */
export function nextPathName(paths: readonly ScenePath[]): string {
  for (let n = paths.length + 1; ; n++) {
    const name = `Path ${n}`;
    if (!paths.some(p => p.name === name)) return name;
  }
}

/** Where a model stands, the point an end anchored to it starts at. */
export function anchorAt(layout: SceneLayout, model: LayoutModel): Vec3 {
  return [...placementOf(fromLayout(layout), model).position];
}

/** Rounds a point to the snap step, or to a tenth of a millimetre. */
export function snapPoint(point: Vec3, snap: boolean): Vec3 {
  const step = SNAP.move;
  return point.map(v => (snap ? round(Math.round(v / step) * step, 4) : round(v, 4))) as Vec3;
}

const withPaths = (draft: PathsDraft, paths: ScenePath[]): PathsDraft => {
  const ids = new Set(paths.map(p => p.id));
  const tracks = Object.fromEntries(Object.entries(draft.tracks).filter(([id]) => ids.has(id)));
  return { paths, tracks };
};

const change = (draft: PathsDraft, id: string, edit: (path: ScenePath) => ScenePath) =>
  withPaths(
    draft,
    draft.paths.map(p => (p.id === id ? edit(p) : p))
  );

/**
 * A new path from `from` to `to`, unanchored, in the default look. Returns
 * the draft and the new path's id.
 */
export function addPath(
  draft: PathsDraft,
  from: Vec3,
  to: Vec3
): { draft: PathsDraft; id: string } {
  const name = nextPathName(draft.paths);
  const id = pathId(draft.paths, name);
  const path: ScenePath = {
    id,
    name,
    curve: 'catmull-rom',
    points: [from, to],
    start: null,
    end: null,
    width: DEFAULT_WIDTH,
    color: DEFAULT_COLOR,
    glow: DEFAULT_GLOW,
  };
  return { draft: withPaths(draft, [...draft.paths, path]), id };
}

export function removePath(draft: PathsDraft, id: string): PathsDraft {
  return withPaths(
    draft,
    draft.paths.filter(p => p.id !== id)
  );
}

/** Changes a path's name, curve or look; the width stays in range. */
export function setPath(
  draft: PathsDraft,
  id: string,
  patch: Partial<Pick<ScenePath, 'name' | 'curve' | 'width' | 'color' | 'glow'>>
): PathsDraft {
  return change(draft, id, p => ({
    ...p,
    ...patch,
    width:
      patch.width === undefined || patch.width === null
        ? p.width
        : clamp(patch.width, MIN_WIDTH, MAX_WIDTH),
  }));
}

/** Adds a point after `after` (the end when null). Returns the draft and its index. */
export function addPoint(
  draft: PathsDraft,
  id: string,
  point: Vec3,
  after: number | null = null
): { draft: PathsDraft; index: number } {
  const path = draft.paths.find(p => p.id === id);
  if (!path || path.points.length >= MAX_POINTS) return { draft, index: -1 };
  const index = after === null ? path.points.length : clamp(after + 1, 0, path.points.length);
  const points = [...path.points];
  points.splice(index, 0, point);
  // A point added past an end takes it over, so that end lets go of its model.
  const end = index === path.points.length ? null : path.end;
  const start = index === 0 ? null : path.start;
  return { draft: change(draft, id, p => ({ ...p, points, start, end })), index };
}

export function movePoint(draft: PathsDraft, id: string, index: number, point: Vec3): PathsDraft {
  return change(draft, id, p => ({
    ...p,
    points: p.points.map((q, i) => (i === index ? point : q)),
  }));
}

/** Removes a point, keeping at least two; an anchored end lets go when it goes. */
export function removePoint(draft: PathsDraft, id: string, index: number): PathsDraft {
  return change(draft, id, p => {
    if (p.points.length <= 2) return p;
    const last = p.points.length - 1;
    return {
      ...p,
      points: p.points.filter((_, i) => i !== index),
      start: index === 0 ? null : p.start,
      end: index === last ? null : p.end,
    };
  });
}

/** Anchors an end to a model (null lets it go); anchoring moves that end to `at`. */
export function setAnchor(
  draft: PathsDraft,
  id: string,
  end: PathEnd,
  asset: string | null,
  at: Vec3 | null
): PathsDraft {
  return change(draft, id, p => {
    const index = end === 'start' ? 0 : p.points.length - 1;
    const points = at ? p.points.map((q, i) => (i === index ? at : q)) : p.points;
    return { ...p, points, [end]: asset };
  });
}

/**
 * Moves each anchored end by how far its model moved from `before` to
 * `after`, as the app does when the layout saves.
 */
export function follow(
  paths: readonly ScenePath[],
  before: Partial<Record<string, Placement>>,
  after: Partial<Record<string, Placement>>
): ScenePath[] {
  return paths.map(path => {
    const points = pointsOf(path);
    const ends: [string | null, number][] = [
      [path.start, 0],
      [path.end, points.length - 1],
    ];
    for (const [asset, index] of ends) {
      const old = asset ? before[asset] : undefined;
      const now = asset ? after[asset] : undefined;
      const point = points[index];
      if (!old || !now || !point) continue;
      const a = vec3(old.position);
      const b = vec3(now.position);
      points[index] = [point[0] + b[0] - a[0], point[1] + b[1] - a[1], point[2] + b[2] - a[2]];
    }
    return { ...path, points };
  });
}

/** How far along the path each point is, in metres, straight runs between them. */
export function along(path: ScenePath): number[] {
  const points = pointsOf(path);
  let total = 0;
  return points.map((p, i) => {
    const q = points[i - 1];
    if (q) total += Math.hypot(p[0] - q[0], p[1] - q[1], p[2] - q[2]);
    return total;
  });
}

export const pathLength = (path: ScenePath) => along(path).at(-1) ?? 0;

/** The triangles the site's tube has: rings about every two widths, 16 to 256 of them. */
export function tubeTris(path: ScenePath): number {
  const step = Math.max(num(path.width) * 2, 0.01);
  const rings = clamp(Math.ceil(pathLength(path) / step), 16, 256);
  return rings * TUBE_SIDES * 2;
}

/** What the path and its track add to layout.json. */
export function pathBytes(path: ScenePath, track: PathTrack | undefined): string {
  return formatBytes(JSON.stringify(path).length + (track ? JSON.stringify(track).length : 0));
}

/** A new energize track: unlit at the first stop, lit at the last. */
export function addEnergize(
  draft: PathsDraft,
  id: string,
  stops: readonly LayoutStop[]
): PathsDraft {
  const first = stops[0];
  const last = stops.at(-1);
  const keys: PathTrack['keys'] = [];
  if (first) keys.push({ stop: first.id, amount: 0 });
  if (last && last !== first) keys.push({ stop: last.id, amount: 1 });
  return { ...draft, tracks: { ...draft.tracks, [id]: { easing: 'linear', keys } } };
}

export function removeEnergize(draft: PathsDraft, id: string): PathsDraft {
  const tracks = { ...draft.tracks };
  delete tracks[id];
  return { ...draft, tracks };
}

/** "0% at Porch, 100% at Pullback", or what an unlit path does. */
export function energizeLine(track: PathTrack | undefined, stops: readonly LayoutStop[]): string {
  const keys = (track?.keys ?? [])
    .map(k => ({ ...k, at: stops.findIndex(s => s.id === k.stop) }))
    .filter(k => k.at >= 0)
    .sort((a, b) => a.at - b.at);
  if (keys.length === 0) return 'Not energized: the path stays unlit';
  return keys
    .map(k => `${Math.round((k.amount ?? 0) * 100)}% at ${stops[k.at]?.name ?? k.stop}`)
    .join(', ');
}

/** Why the draft cannot be saved, or null. */
export function pathsProblem(draft: PathsDraft): string | null {
  for (const path of draft.paths) {
    const name = path.name.trim();
    if (!name) return 'Give every path a name.';
    if (name.length > PATH_NAME_MAX) return `Keep the path name ${name} to 60 characters.`;
  }
  return null;
}

/**
 * A recipe's timeline: which frame of which clip each frame of its bake
 * shows. A copy of the core's rules (crates/maquette-model/src/recipes/
 * timeline.rs), step for step, so the rough preview and the lanes play the
 * frames the bake makes; fixtures/timeline.json there holds the cases both
 * agree on.
 *
 * Each clip is cut to its trim, then its time edits play out on its own
 * frames (a retime steps faster or slower, a freeze stops and adds frames, a
 * hold over keeps a pose with the length unchanged), then the clips join in
 * order across their blend windows. Round 1's changes keep their shares, read
 * on the clips' own frames laid end to end (`share`).
 */
import type { BlendCurve, LineUp, RecipeBase, RecipeFile, TimeEdit } from '@/ipc/client';

/** How long one breath of a held pose lasts, seconds. */
export const BREATHE_SECONDS = 3;
const REST_FPS = 30;
const EPS = 1e-6;
const MAX_KEYS = 200_000;

export interface HoldSample {
  at: number;
  /** How much of the held pose shows over the motion, 0 to 1. */
  weight: number;
  breathe: number;
}

/** A frame of one clip, before its trim; between keys when a retime steps part of a frame. */
export interface ClipSample {
  clip: number;
  at: number;
  hold?: HoldSample;
}

export interface TimelineKey {
  from: ClipSample;
  /** In a blend window, the next clip's frame. */
  to?: ClipSample;
  /** How much of `to` shows, 0 to 1. */
  blend: number;
  /** Where round 1's changes stand: the share of the clips' own frames end to end. */
  share: number;
}

export interface Seam {
  clip: number;
  frame: number;
  frames: number;
  lineUp: LineUp;
  /** The clip before's frame and the clip's own first frame at `frame`, which the line up compares. */
  from: ClipSample;
  to: ClipSample;
}

export interface ClipSpan {
  clip: number;
  first: number;
  last: number;
}

export interface Timeline {
  /** The base clip's rate. */
  fps: number;
  /** The last frame: keys run 0 to `frames`. */
  frames: number;
  keys: TimelineKey[];
  seams: Seam[];
  clips: ClipSpan[];
  mirror: boolean;
}

const smooth = (x: number) => {
  const c = Math.min(Math.max(x, 0), 1);
  return c * c * (3 - 2 * c);
};

export const clipsOf = (recipe: RecipeFile): RecipeBase[] => [
  recipe.base,
  ...(recipe.joined ?? []),
];

/** A clip's frames a second, to a thousandth: its frames over its length. */
export function clipFps(clip: RecipeBase): number {
  const length = clip.length ?? 0;
  if (clip.rest || clip.frames === 0 || !(length > 0)) return REST_FPS;
  return Math.round((clip.frames / length) * 1000) / 1000;
}

function speedAt(retimes: readonly TimeEdit[], p: number): number {
  for (const r of retimes) {
    if (r.from == null || r.to == null || r.speed == null) continue;
    const { from, to, speed: s } = r;
    if (p < from || p >= to) continue;
    const ease = r.ease ?? 0;
    if (ease > 0 && p < from + ease) return 1 + (s - 1) * ((p - from) / ease);
    if (ease > 0 && p > to - ease) return 1 + (s - 1) * ((to - p) / ease);
    return s;
  }
  return 1;
}

function holdWeight(h: TimeEdit, at: number, n: number, p: number): number {
  const easeIn = h.easeIn ?? 0;
  const easeOut = h.easeOut ?? 0;
  if (h.kind === 'hold-over') {
    if (p < at - EPS || p > at + n + EPS) return 0;
    const k = Math.min(Math.max(p - at, 0), n);
    const wIn = easeIn > 0 ? smooth(k / easeIn) : 1;
    const wOut = easeOut > 0 ? smooth((n - k) / easeOut) : k < n - EPS ? 1 : 0;
    return Math.min(wIn, wOut);
  }
  if (h.kind === 'freeze') {
    if (Math.abs(p - at) < EPS) return 1;
    if (easeIn > 0 && p >= at - easeIn && p < at) return smooth((p - (at - easeIn)) / easeIn);
    if (easeOut > 0 && p > at && p <= at + easeOut) return smooth((at + easeOut - p) / easeOut);
  }
  return 0;
}

function holdAt(holds: readonly TimeEdit[], p: number): HoldSample | undefined {
  let best: HoldSample | undefined;
  for (const h of holds) {
    if (h.at == null || h.frames == null) continue;
    const weight = holdWeight(h, h.at, h.frames, p);
    if (weight > EPS && (!best || weight > best.weight))
      best = { at: h.at, weight, breathe: h.breathe ?? 0 };
  }
  return best;
}

function clipSamples(recipe: RecipeFile, index: number, fps: number): ClipSample[] {
  const clip = clipsOf(recipe)[index];
  if (!clip) return [];
  const [a, b] = clip.trim ?? [0, clip.frames];
  const rate = Math.min(Math.max(clipFps(clip) / fps, 0.1), 10);
  const mine = (recipe.time ?? []).filter(t => t.clip === index);
  const retimes = mine.filter(t => t.kind === 'retime');
  const holds = mine.filter(t => t.kind !== 'retime');
  const freezes = mine
    .filter(t => t.kind === 'freeze' && t.at != null && t.frames != null)
    .filter(t => (t.at ?? 0) >= a && (t.at ?? 0) <= b)
    .sort((x, y) => (x.at ?? 0) - (y.at ?? 0));
  const emitted = freezes.map(() => false);
  const out: ClipSample[] = [];
  const push = (p: number) => {
    const hold = holdAt(holds, p);
    out.push(hold ? { clip: index, at: p, hold } : { clip: index, at: p });
    freezes.forEach((f, k) => {
      const at = f.at ?? 0;
      if (emitted[k] || Math.abs(p - at) >= EPS) return;
      emitted[k] = true;
      const held: HoldSample = { at, weight: 1, breathe: f.breathe ?? 0 };
      for (let i = 0; i < (f.frames ?? 0); i += 1) out.push({ clip: index, at, hold: { ...held } });
    });
  };
  let p = a;
  push(p);
  while (p < b - EPS && out.length < MAX_KEYS) {
    let next = p + speedAt(retimes, p) * rate;
    const k = freezes.findIndex((f, i) => {
      const at = f.at ?? 0;
      return !emitted[i] && p + EPS < at && at < next - EPS;
    });
    if (k >= 0) next = freezes[k]?.at ?? next;
    p = Math.min(next, b);
    push(p);
  }
  return out;
}

const curve = (kind: BlendCurve, x: number) =>
  kind === 'linear' ? Math.min(Math.max(x, 0), 1) : smooth(x);

const len = (v: readonly ClipSample[]) => Math.max(v.length - 1, 0);

/** The timeline of `recipe`. */
export function timeline(recipe: RecipeFile): Timeline {
  const clips = clipsOf(recipe);
  const fps = clipFps(recipe.base);
  const parts = clips.map((_, i) => clipSamples(recipe, i, fps));
  const starts = parts.map(() => 0);
  const windows = parts.map(() => 0);
  for (let i = 1; i < parts.length; i += 1) {
    const prev = parts[i - 1] ?? [];
    const wanted = clips[i]?.blend?.frames ?? 0;
    const room = Math.max(len(prev) - (windows[i - 1] ?? 0), 0);
    windows[i] = Math.min(wanted, room, len(parts[i] ?? []));
    starts[i] = (starts[i - 1] ?? 0) + len(prev) - (windows[i] ?? 0);
  }
  const last = parts.length - 1;
  const total = (starts[last] ?? 0) + len(parts[last] ?? []);
  const before: number[] = [];
  let sum = 0;
  for (const c of clips) {
    before.push(sum);
    sum += c.frames;
  }
  const place = (s: ClipSample) => (before[s.clip] ?? 0) + s.at;
  const share = (x: number) => (sum > 0 ? Math.min(Math.max(x / sum, 0), 1) : 0);
  const keys: TimelineKey[] = [];
  for (let t = 0; t <= total; t += 1) {
    let i = parts.length - 1;
    while (i > 0 && (starts[i] ?? 0) > t) i -= 1;
    const own = (parts[i] ?? [])[t - (starts[i] ?? 0)] as ClipSample;
    const window = windows[i] ?? 0;
    if (i > 0 && t < (starts[i] ?? 0) + window) {
      const from = (parts[i - 1] ?? [])[t - (starts[i - 1] ?? 0)] as ClipSample;
      const w = curve(clips[i]?.blend?.curve ?? 'smooth', (t - (starts[i] ?? 0)) / window);
      keys.push({ from, to: own, blend: w, share: share((1 - w) * place(from) + w * place(own)) });
    } else keys.push({ from: own, blend: 0, share: share(place(own)) });
  }
  return {
    fps,
    frames: total,
    keys,
    seams: parts.slice(1).map((part, k) => {
      const at = starts[k + 1] ?? 0;
      return {
        clip: k + 1,
        frame: at,
        frames: windows[k + 1] ?? 0,
        lineUp: clips[k + 1]?.blend?.lineUp ?? 'hips-and-facing',
        from: (parts[k] ?? [])[at - (starts[k] ?? 0)] as ClipSample,
        to: part[0] as ClipSample,
      };
    }),
    clips: parts.map((p, i) => ({
      clip: i,
      first: starts[i] ?? 0,
      last: (starts[i] ?? 0) + len(p),
    })),
    mirror: Boolean(recipe.mirror),
  };
}

/** Whether `recipe` plays its one clip whole, as round 1 did. */
export const plain = (recipe: RecipeFile) =>
  !recipe.joined?.length && !recipe.base.trim && !recipe.time?.length && !recipe.mirror;

/** The timeline frame nearest where round 1's changes stand at `share`. */
export function frameOfShare(t: Timeline, share: number): number {
  let best = 0;
  let gap = Infinity;
  t.keys.forEach((k, i) => {
    const d = Math.abs(k.share - share);
    if (d >= gap - EPS) return;
    gap = d;
    best = i;
  });
  return best;
}

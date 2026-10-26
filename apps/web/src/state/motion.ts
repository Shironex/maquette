/**
 * The Motion view's logic: which models move, what their clips weigh inside
 * the scene budget, the draft of object states and clip choices, and the
 * numbers the timeline shows. Plain functions, so the tests need no
 * renderer.
 */
import type { Clip, MotionModel, MotionView, ObjectStates } from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { formatBytes } from '@/lib/format';

export type MotionFilter = 'all' | 'characters' | 'props';
export type MotionTab = 'clips' | 'states';

/** What Save writes: every asset's states and the clips kept off the site. */
export interface MotionDraft {
  states: Record<string, ObjectStates>;
  clipsOff: Record<string, string[]>;
}

export const EMPTY_DRAFT: MotionDraft = { states: {}, clipsOff: {} };
/** Rows the list shows of the models that do not move, before Show more. */
export const STILL_ROWS = 3;
export const SPEEDS = ['0.25', '0.5', '1', '2'] as const;
export type Speed = (typeof SPEEDS)[number];

/** A number the bindings send as `number | null` (Rust floats). */
export function n(value: number | null | undefined): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

export function draftOf(view: MotionView): MotionDraft {
  const clipsOff: Record<string, string[]> = {};
  for (const [asset, names] of Object.entries(view.clipsOff)) {
    if (names.length > 0) clipsOff[asset] = [...names].sort();
  }
  return { states: structuredClone(view.states), clipsOff };
}

export function sameDraft(a: MotionDraft, b: MotionDraft): boolean {
  return deepEqual(a, b);
}

/** A model moves when it has a rig, clips or states. */
export function moves(model: MotionModel, draft: MotionDraft): boolean {
  return Boolean(model.rig) || model.clips.length > 0 || stateCount(model, draft) > 0;
}

export function stateCount(model: MotionModel, draft: MotionDraft): number {
  return draft.states[model.asset]?.states.length ?? 0;
}

export function matchesFilter(model: MotionModel, filter: MotionFilter): boolean {
  if (filter === 'characters') return model.kind === 'character';
  if (filter === 'props') return model.kind !== 'character';
  return true;
}

const plural = (count: number, one: string, many = `${one}s`) =>
  `${count} ${count === 1 ? one : many}`;

/** "52 bones · 4 clips", "no rig · 2 states", "8 parts · no rig". */
export function modelLine(model: MotionModel, draft: MotionDraft): string {
  const rig = model.rig ? plural(model.rig.joints, 'bone') : 'no rig';
  const states = stateCount(model, draft);
  if (model.rig || model.clips.length > 0) {
    const second =
      model.clips.length > 0 || !states
        ? plural(model.clips.length, 'clip')
        : plural(states, 'state');
    return `${rig} · ${second}`;
  }
  if (states > 0) return `${rig} · ${plural(states, 'state')}`;
  return `${plural(model.parts.length, 'part')} · ${rig}`;
}

export function isOff(draft: MotionDraft, asset: string, clip: string): boolean {
  return draft.clipsOff[asset]?.includes(clip) ?? false;
}

/** The draft with `clip` shipping or kept off the site. */
export function withShip(
  draft: MotionDraft,
  asset: string,
  clip: string,
  ship: boolean
): MotionDraft {
  const off = new Set(draft.clipsOff[asset] ?? []);
  if (ship) off.delete(clip);
  else off.add(clip);
  const clipsOff = { ...draft.clipsOff };
  if (off.size > 0) clipsOff[asset] = [...off].sort();
  else delete clipsOff[asset];
  return { ...draft, clipsOff };
}

/** What a clip weighs in the file the model is weighed by. */
export function clipBytes(clip: Clip): number {
  return clip.shippedBytes ?? n(clip.bytes);
}

/** The bytes a model's clips add once the ones kept off are gone. */
export function shippedMotion(model: MotionModel, draft: MotionDraft): number {
  const off = model.clips.filter(c => isOff(draft, model.asset, c.name));
  return Math.max(0, n(model.motionBytes) - off.reduce((sum, c) => sum + clipBytes(c), 0));
}

export interface MotionBudgetLine {
  /** Megabytes the scene loads with the draft's clip choices. */
  sceneMb: number;
  targetMb: number;
  /** Bytes of clips among them. */
  motion: number;
  over: boolean;
  /** Bytes over the scene budget, 0 when it fits. */
  overBy: number;
  firstViewOver: boolean;
  firstViewMotion: number;
}

/**
 * The scene budget with the clips in it: motion has no budget of its own,
 * its bytes are part of the models. Clips kept off the site in the draft
 * leave the scene (Build scene drops them on the next run).
 */
export function budgetLine(view: MotionView, draft: MotionDraft): MotionBudgetLine {
  const removed = view.models.reduce(
    (sum, m) => sum + (n(m.motionBytes) - shippedMotion(m, draft)),
    0
  );
  const scene = Math.max(0, n(view.budget.sceneBytes) - removed);
  const target = n(view.budget.sceneTargetMb) || 3;
  const motion = Math.max(0, n(view.budget.motionBytes) - removed);
  const fv = view.budget.firstView;
  const firstViewIds = new Set(fv.models.map(m => m.asset));
  const fvRemoved = view.models
    .filter(m => firstViewIds.has(m.asset))
    .reduce((sum, m) => sum + (n(m.motionBytes) - shippedMotion(m, draft)), 0);
  const fvBytes = Math.max(0, n(fv.bytes) - fvRemoved);
  return {
    sceneMb: scene / 1e6,
    targetMb: target,
    motion,
    over: scene > target * 1e6,
    overBy: Math.max(0, scene - target * 1e6),
    firstViewOver: fvBytes > (n(fv.targetMb) || 3) * 1e6,
    firstViewMotion: Math.max(0, n(view.budget.firstViewMotion) - fvRemoved),
  };
}

/** The tooltip under the scene meter: how much of it is motion. */
export function budgetTip(line: MotionBudgetLine): string {
  const share = `Clips are ${formatBytes(line.motion)} of it`;
  const first = line.firstViewMotion
    ? `, ${formatBytes(line.firstViewMotion)} of them load with the first camera stop`
    : '';
  return `The whole scene against its budget, models and clips together. ${share}${first}.`;
}

/** The clips of `model` that ship, heaviest first: the ones worth leaving off. */
export function heaviestClips(model: MotionModel, draft: MotionDraft): Clip[] {
  return model.clips
    .filter(c => !isOff(draft, model.asset, c.name))
    .sort((a, b) => clipBytes(b) - clipBytes(a));
}

export interface ClipToFit {
  model: MotionModel;
  clip: Clip;
  bytes: number;
  /** With this clip and the heavier ones off, the scene fits its budget. */
  fits: boolean;
}

/**
 * The clips that ship across the scene, heaviest first, up to the one that
 * makes the scene fit once it and the ones before it stay off the site.
 * Empty while the scene fits.
 */
export function clipsToFit(view: MotionView, draft: MotionDraft): ClipToFit[] {
  const { overBy } = budgetLine(view, draft);
  if (overBy <= 0) return [];
  const shipping = view.models
    .flatMap(model =>
      heaviestClips(model, draft).map(clip => ({ model, clip, bytes: clipBytes(clip) }))
    )
    .sort((a, b) => b.bytes - a.bytes);
  const out: ClipToFit[] = [];
  let saved = 0;
  for (const item of shipping) {
    saved += item.bytes;
    out.push({ ...item, fits: saved >= overBy });
    if (saved >= overBy) break;
  }
  return out;
}

/** The frame at `time` seconds, at the clip's key rate. */
export function frameAt(clip: Clip, time: number): number {
  return Math.round(time * n(clip.fps));
}

export function timeOfFrame(clip: Clip, frame: number): number {
  const fps = n(clip.fps);
  return fps > 0 ? frame / fps : 0;
}

/** "18 / 32 f · 0.60 / 1.07 s" */
export function readout(clip: Clip, time: number): string {
  const duration = n(clip.duration);
  return `${frameAt(clip, time)} / ${clip.frames} f · ${time.toFixed(2)} / ${duration.toFixed(2)} s`;
}

/** Where the playhead goes next while playing: on to the end, round again inside the range. */
export function advance(
  time: number,
  delta: number,
  range: [number, number],
  loop: boolean
): { time: number; playing: boolean } {
  const [from, to] = range;
  const next = time + delta;
  if (next < to) return { time: Math.max(next, from), playing: true };
  if (!loop) return { time: to, playing: false };
  const span = to - from;
  return { time: span > 0 ? from + ((next - from) % span) : from, playing: true };
}

/** Ruler ticks across `frames`: about six, on round numbers. */
export function ticks(frames: number): number[] {
  if (frames <= 0) return [0];
  const steps = [1, 2, 4, 5, 8, 10, 20, 25, 30, 40, 50, 60, 100, 120, 200, 250, 500];
  const step = steps.find(s => frames / s <= 6) ?? Math.ceil(frames / 6);
  const out: number[] = [];
  for (let f = 0; f <= frames; f += step) out.push(f);
  if (out[out.length - 1] !== frames) out.push(frames);
  return out;
}

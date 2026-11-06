/**
 * A recipe's time edits as the editor shows and makes them: a new retime,
 * freeze or hold over with starting values, their words, and where each
 * one lands on the bake's frames through the timeline. Time edits sit on
 * their clip's own frames, before its trim; the lanes show them on the
 * output ruler. Pure helpers.
 */
import type { RecipeBase, RecipeFile, TimeEdit, TimeKind } from '@/ipc/client';
import type { ClipSample, Timeline } from './recipe-timeline';
import { timeline } from './recipe-timeline';

export const TIME_WORDS: Record<TimeKind, string> = {
  retime: 'Retime',
  freeze: 'Freeze',
  'hold-over': 'Hold over',
};

/** The retime speeds offered as one click. */
export const SPEED_PRESETS = [0.5, 0.75, 1.25, 1.6, 2] as const;

const EPS = 1e-6;

/** "1.6x", without trailing zeros. */
export const speedWords = (speed: number | null | undefined) =>
  `${Number((speed ?? 1).toFixed(2))}x`;

/** A new retime on `clip` (index `index`) over a fifth of its kept frames, a little faster. */
export function newRetime(clip: RecipeBase, index: number): TimeEdit {
  const [a, b] = clip.trim ?? [0, clip.frames];
  const span = Math.max(Math.round((b - a) / 5), 2);
  const from = Math.min(a + Math.round((b - a) / 4), Math.max(b - span, a));
  return { kind: 'retime', clip: index, from, to: Math.min(from + span, b), speed: 1.6, ease: 0 };
}

/** A new freeze or hold over in the middle of the kept frames of `clip`. */
export function newHold(kind: 'freeze' | 'hold-over', clip: RecipeBase, index: number): TimeEdit {
  const [a, b] = clip.trim ?? [0, clip.frames];
  const at = Math.min(a + Math.round((b - a) / 2), Math.max(clip.frames - 1, 0));
  const frames = kind === 'freeze' ? 12 : Math.max(Math.min(12, clip.frames - at), 1);
  return { kind, clip: index, at, frames, breathe: 0.3, easeIn: 3, easeOut: 3 };
}

/** A time edit as its kind writes it: only the fields it reads. */
export function asKind(edit: TimeEdit, kind: TimeKind, clip: RecipeBase): TimeEdit {
  if (edit.kind === kind) return edit;
  if (kind === 'retime') return newRetime(clip, edit.clip);
  if (edit.kind === 'retime') return newHold(kind, clip, edit.clip);
  const frames =
    kind === 'hold-over'
      ? Math.max(Math.min(edit.frames ?? 1, clip.frames - (edit.at ?? 0)), 1)
      : edit.frames;
  return { ...edit, kind, frames };
}

/** "Retime 1.6x", "Freeze 18 f", "Hold over 12 f". */
export function timeTitle(edit: TimeEdit): string {
  if (edit.kind === 'retime') return `${TIME_WORDS.retime} ${speedWords(edit.speed)}`;
  return `${TIME_WORDS[edit.kind]} ${edit.frames ?? 0} f`;
}

/** Whether a sample of the bake shows the part of its clip `edit` covers. */
function covers(edit: TimeEdit, s: ClipSample): boolean {
  if (s.clip !== edit.clip) return false;
  if (edit.kind === 'retime') return s.at >= (edit.from ?? 0) - EPS && s.at <= (edit.to ?? 0) + EPS;
  const at = edit.at ?? 0;
  if (edit.kind === 'freeze') return Math.abs(s.at - at) < EPS;
  return s.at >= at - EPS && s.at <= at + (edit.frames ?? 0) + EPS;
}

/** The first and last frame of the bake that show what `edit` covers, or null when none does. */
export function editSpan(t: Timeline, edit: TimeEdit): [number, number] | null {
  let first = -1;
  let last = -1;
  t.keys.forEach((k, i) => {
    if (!covers(edit, k.from) && !(k.to && covers(edit, k.to))) return;
    if (first < 0) first = i;
    last = i;
  });
  return first < 0 ? null : [first, last];
}

/** The bake frame nearest frame `at` of clip `clip`, or null when its trim cuts it. */
export function frameOfClip(t: Timeline, clip: number, at: number): number | null {
  let best: number | null = null;
  let gap = Infinity;
  t.keys.forEach((k, i) => {
    for (const s of [k.from, k.to]) {
      if (!s || s.clip !== clip) continue;
      const d = Math.abs(s.at - at);
      if (d < gap - EPS) {
        gap = d;
        best = i;
      }
    }
  });
  return gap <= 1 + EPS ? best : null;
}

/** How many frames longer the recipe is with time edit `index` than without it. */
export function lengthChange(recipe: RecipeFile, index: number): number {
  const time = recipe.time ?? [];
  const without = { ...recipe, time: time.filter((_, i) => i !== index) };
  return timeline(recipe).frames - timeline(without).frames;
}

/** "5 f shorter", "3 f longer", "same length". */
export function changeWords(change: number): string {
  if (change === 0) return 'same length';
  return `${Math.abs(change)} f ${change < 0 ? 'shorter' : 'longer'}`;
}

/** How many of the bake's frames a retime's range plays in: "14 frames play in 9 f". */
export function playsIn(t: Timeline, edit: TimeEdit): string {
  const reached = (frame: number) =>
    t.keys.findIndex(k => [k.from, k.to].some(s => s?.clip === edit.clip && s.at >= frame - EPS));
  const first = reached(edit.from ?? 0);
  const end = reached(edit.to ?? 0);
  const played = first < 0 ? 0 : (end < 0 ? t.frames : end) - first;
  return `${(edit.to ?? 0) - (edit.from ?? 0)} frames play in ${played} f`;
}

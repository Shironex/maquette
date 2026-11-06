/**
 * The core's checks of what recipe file version 2 adds to the clips and the
 * time edits (crates/maquette-model/src/recipes/check.rs), said per clip and
 * per time edit so the editor marks each field before Save. The mock backend
 * reads the first problem of the same lists, so both say the same words.
 */
import type { BlendWindow, RecipeBase, TimeEdit } from '@/ipc/client';

/** The core's limits of version 2. */
export const V2_LIMITS = {
  clips: 8,
  keys: 64,
  keyBones: 32,
  gripBones: 64,
  timeEdits: 32,
  recipeKeys: 256,
  recipeTurns: 1024,
  holdFrames: 600,
  turnDegrees: 180,
  minSpeed: 0.25,
  maxSpeed: 4,
} as const;

/** A joined clip's blend unless it says otherwise: 8 frames, smooth, hips and facing lined up. */
export const DEFAULT_BLEND: BlendWindow = { frames: 8, curve: 'smooth', lineUp: 'hips-and-facing' };

/** How many frames of `clip` its trim keeps. */
export const keptFrames = (clip: RecipeBase) =>
  clip.trim ? clip.trim[1] - clip.trim[0] : clip.frames;

/** The longest blend into clip `i`: half the shorter of it and the clip before, as trimmed. */
export function blendMost(clips: readonly RecipeBase[], i: number): number {
  const before = clips[i - 1];
  const clip = clips[i];
  if (!before || !clip) return 0;
  return Math.floor(Math.min(keptFrames(before), keptFrames(clip)) / 2);
}

/** What is wrong with the clips as a whole, before any one of them. */
export function clipsGeneral(clips: readonly RecipeBase[]): string | null {
  if (clips.length > V2_LIMITS.clips) return `A recipe joins at most ${V2_LIMITS.clips} clips.`;
  if (clips[0]?.rest && clips.length > 1)
    return 'A recipe on the rest pose has no clips to join. Start it on a clip instead.';
  return null;
}

/** What is wrong with each clip's trim (or its being the rest pose), by clip. */
export function trimProblems(clips: readonly RecipeBase[]): (string | null)[] {
  return clips.map((clip, i) => {
    if (i > 0 && clip.rest)
      return `Only the base can be the rest pose, and then alone; clip ${i + 1} cannot.`;
    if (!clip.trim) return null;
    if (clip.rest) return 'The rest pose has no clip to trim. Set how many frames it runs instead.';
    const [from, to] = clip.trim;
    if (from >= to || to > clip.frames)
      return `The clip ${i + 1} keeps frames ${from} to ${to}; keep at least one frame within its ${clip.frames} frames, the first before the last.`;
    return null;
  });
}

/** What is wrong with the blend into each clip, by clip (never the base). */
export function blendProblems(clips: readonly RecipeBase[]): (string | null)[] {
  return clips.map((clip, i) => {
    if (i === 0) return null;
    const most = blendMost(clips, i);
    const frames = clip.blend?.frames ?? DEFAULT_BLEND.frames;
    if (frames > most)
      return `The blend into clip ${i + 1} lasts ${frames} frames; it can last at most ${most}, half the shorter clip as trimmed.`;
    return null;
  });
}

function editProblem(edit: TimeEdit, clips: readonly RecipeBase[]): string | null {
  const clip = clips[edit.clip];
  if (!clip) return `A time edit sits on clip ${edit.clip + 1}, which the recipe does not have.`;
  const where = `clip ${edit.clip + 1}`;
  if (edit.kind === 'retime') {
    const { from, to, speed } = edit;
    if (from == null || to == null || speed == null)
      return `A retime on ${where} needs from, to and speed.`;
    if (from >= to || to > clip.frames)
      return `A retime on ${where} runs from frame ${from} to ${to}; keep it within its ${clip.frames} frames, from before to.`;
    if (speed < V2_LIMITS.minSpeed || speed > V2_LIMITS.maxSpeed)
      return `A retime plays 0.25x to 4x, not ${speed}x.`;
    return null;
  }
  const word = edit.kind === 'freeze' ? 'freeze' : 'hold over';
  const { at, frames } = edit;
  if (at == null || frames == null) return `A ${word} on ${where} needs at and frames.`;
  if (at >= clip.frames)
    return `A ${word} on ${where} holds frame ${at}, past its ${clip.frames} frames.`;
  if (frames < 1 || frames > V2_LIMITS.holdFrames)
    return `A ${word} lasts 1 to ${V2_LIMITS.holdFrames} frames, not ${frames}.`;
  if (edit.kind === 'hold-over' && at + frames > clip.frames)
    return `A hold over on ${where} keeps the length, so it has to end within its ${clip.frames} frames.`;
  return null;
}

/** The frames a time edit covers on its clip, end not included. */
const spanOf = (edit: TimeEdit): [number, number] =>
  edit.kind === 'retime'
    ? [edit.from ?? 0, edit.to ?? 0]
    : [
        edit.at ?? 0,
        edit.kind === 'hold-over' ? (edit.at ?? 0) + (edit.frames ?? 0) : (edit.at ?? 0) + 1,
      ];

/** What is wrong with the time edits as a whole. */
export const timeGeneral = (time: readonly TimeEdit[]): string | null =>
  time.length > V2_LIMITS.timeEdits
    ? `A recipe holds at most ${V2_LIMITS.timeEdits} time edits.`
    : null;

/** What is wrong with each time edit on its own, by edit. */
export const editProblems = (time: readonly TimeEdit[], clips: readonly RecipeBase[]) =>
  time.map(edit => editProblem(edit, clips));

/**
 * Each time edit that overlaps one before it of its sort (retimes with
 * retimes, holds with holds) on the same clip, by edit. Only edits with
 * nothing else wrong count, as the core checks them.
 */
export function overlapProblems(
  time: readonly TimeEdit[],
  clips: readonly RecipeBase[]
): (string | null)[] {
  const fine = editProblems(time, clips).map(p => p === null);
  return time.map((edit, i) => {
    if (!fine[i]) return null;
    const [first, end] = spanOf(edit);
    const clash = time.slice(0, i).some((o, k) => {
      if (!fine[k] || o.clip !== edit.clip) return false;
      if ((o.kind === 'retime') !== (edit.kind === 'retime')) return false;
      const [a, b] = spanOf(o);
      return a < end && first < b;
    });
    if (!clash) return null;
    return edit.kind === 'retime'
      ? `Two retime ranges on clip ${edit.clip + 1} overlap. Keep each range apart.`
      : `Two holds on clip ${edit.clip + 1} overlap. Keep each hold apart.`;
  });
}

const firstOf = (list: readonly (string | null)[]) => list.find(Boolean) ?? null;

/** The first problem of the clips, in the order the core checks them. */
export const clipsProblem = (clips: readonly RecipeBase[]): string | null =>
  clipsGeneral(clips) ?? firstOf(trimProblems(clips)) ?? firstOf(blendProblems(clips));

/** The first problem of the time edits, in the order the core checks them. */
export const timeProblem = (time: readonly TimeEdit[], clips: readonly RecipeBase[]) =>
  timeGeneral(time) ?? firstOf(editProblems(time, clips)) ?? firstOf(overlapProblems(time, clips));

/** What to fix per clip and per time edit, for the editor's fields. */
export interface BaseProblems {
  general: string[];
  /** By clip: its trim, or the blend into it. */
  clips: (string | null)[];
  time: (string | null)[];
}

export function baseProblems(
  clips: readonly RecipeBase[],
  time: readonly TimeEdit[]
): BaseProblems {
  const trims = trimProblems(clips);
  const blends = blendProblems(clips);
  const edits = editProblems(time, clips);
  const overlaps = overlapProblems(time, clips);
  return {
    general: [clipsGeneral(clips), timeGeneral(time)].filter((p): p is string => Boolean(p)),
    clips: clips.map((_, i) => trims[i] ?? blends[i] ?? null),
    time: time.map((_, i) => edits[i] ?? overlaps[i] ?? null),
  };
}

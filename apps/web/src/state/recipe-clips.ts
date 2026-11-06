/**
 * A recipe's clips as the editor changes them: joining a library clip,
 * taking one out, moving one, trimming and blending, and the mirror's
 * swap of sides. Each keeps the draft whole: the base has no blend, every
 * joined clip has one, `kept` says which clip of the file as read each one
 * keeps (null for a clip newly picked) and the time edits follow their
 * clip's place. Pure helpers; the store is use-recipe-editor-store.ts.
 */
import type {
  AnimationClip,
  BlendWindow,
  RecipeBase,
  RecipeFile,
  RecipeLayer,
  TimeEdit,
} from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import type { RecipeDraft } from './recipe-draft';
import { DEFAULT_BLEND, blendMost } from './recipe-v2-check';

/** A library clip as a recipe keeps it: its name, frames, length and hash, never its keys. */
export const clipOfLibrary = (clip: AnimationClip): RecipeBase => ({
  name: clip.name,
  frames: clip.frames,
  length: clip.length ?? 0,
  hash: clip.hash,
  mixamo: clip.rig.kind === 'mixamo',
});

/** The core's match of a clip it keeps: the same name, frames, hash and kind. */
export const sameClip = (a: RecipeBase, b: RecipeBase) =>
  a.name === b.name &&
  a.frames === b.frames &&
  a.hash === b.hash &&
  Boolean(a.rest) === Boolean(b.rest) &&
  Boolean(a.byName) === Boolean(b.byName);

/** The starting blend into clip `i`: the default, shortened to what the clips allow. */
export function startBlend(clips: readonly RecipeBase[], i: number): BlendWindow {
  return { ...DEFAULT_BLEND, frames: Math.min(DEFAULT_BLEND.frames, blendMost(clips, i)) };
}

/** `clips` with no blend on the base and one on every joined clip. */
function blended(clips: readonly RecipeBase[]): RecipeBase[] {
  return clips.map((clip, i) => {
    if (i === 0) {
      if (!('blend' in clip)) return clip;
      const bare = { ...clip };
      delete bare.blend;
      return bare;
    }
    return clip.blend ? clip : { ...clip, blend: startBlend(clips, i) };
  });
}

/** Whether the clips or which clip each keeps differ from the file. */
export const clipsChanged = (draft: RecipeDraft, file: RecipeFile) =>
  !deepEqual(draft.clips, [file.base, ...(file.joined ?? [])]) ||
  draft.kept.some((k, i) => k !== i);

/** Joins `clip` after the last one. */
export function addClip(draft: RecipeDraft, clip: RecipeBase): RecipeDraft {
  return {
    ...draft,
    clips: blended([...draft.clips, clip]),
    kept: [...draft.kept, null],
  };
}

/** What the core says when a clip taken out still holds time edits. */
export function inUse(draft: RecipeDraft, index: number): string | null {
  const count = draft.time.filter(t => t.clip === index).length;
  if (!count) return null;
  const clip = draft.clips[index];
  const edits = count === 1 ? 'a time edit sits on it' : `${count} time edits sit on it`;
  return `Clip ${index + 1} (${clip?.name ?? ''}) cannot go: ${edits}. Take those out first.`;
}

/** The draft without clip `index`, or why it cannot go. */
export function removeClip(
  draft: RecipeDraft,
  index: number
): { draft: RecipeDraft } | { problem: string } {
  if (draft.clips.length < 2) return { problem: 'A recipe needs a clip.' };
  const problem = inUse(draft, index);
  if (problem) return { problem };
  return {
    draft: {
      ...draft,
      clips: blended(draft.clips.filter((_, i) => i !== index)),
      kept: draft.kept.filter((_, i) => i !== index),
      time: draft.time.map(t => (t.clip > index ? { ...t, clip: t.clip - 1 } : t)),
    },
  };
}

/** The draft with clip `from` moved to place `to`; its time edits go with it. */
export function moveClip(draft: RecipeDraft, from: number, to: number): RecipeDraft {
  const n = draft.clips.length;
  if (from === to || from < 0 || to < 0 || from >= n || to >= n) return draft;
  const order = draft.clips.map((_, i) => i);
  order.splice(to, 0, ...order.splice(from, 1));
  const placeOf = new Map(order.map((old, place) => [old, place]));
  return {
    ...draft,
    clips: blended(order.map(i => draft.clips[i] as RecipeBase)),
    kept: order.map(i => draft.kept[i] ?? null),
    time: draft.time.map(t => ({ ...t, clip: placeOf.get(t.clip) ?? t.clip })),
  };
}

/** Clip `index` with `patch`; a trim of the whole clip is no trim. */
export function patchClip(
  draft: RecipeDraft,
  index: number,
  patch: Partial<Pick<RecipeBase, 'trim' | 'blend'>>
): RecipeDraft {
  return {
    ...draft,
    clips: draft.clips.map((clip, i) => {
      if (i !== index) return clip;
      const next = { ...clip, ...patch };
      if (next.trim && next.trim[0] === 0 && next.trim[1] === clip.frames) delete next.trim;
      if (next.trim === null) delete next.trim;
      return next;
    }),
  };
}

/**
 * Which clip of `now` (the file as read now) each clip of the draft keeps,
 * once the file under the draft changed from `was`: the clip it kept, if
 * `now` has it in the same place, else the first unused clip that matches.
 */
export function rekeep(
  clips: readonly RecipeBase[],
  kept: readonly (number | null)[],
  was: readonly RecipeBase[],
  now: readonly RecipeBase[]
): (number | null)[] {
  const taken = now.map(() => false);
  const first = kept.map((k, j) => {
    const old = k == null ? undefined : was[k];
    const clip = clips[j];
    if (k == null || !old || !clip) return null;
    const same = now[k];
    if (same && sameClip(same, clip) && !taken[k]) {
      taken[k] = true;
      return k;
    }
    return undefined;
  });
  return first.map((k, j) => {
    if (k !== undefined) return k;
    const clip = clips[j];
    const i = now.findIndex((c, n) => !taken[n] && clip && sameClip(c, clip));
    if (i < 0) return null;
    taken[i] = true;
    return i;
  });
}

/** The recipe the draft makes of `file`: its clips, time edits, mirror, changes and blanks. */
export function recipeOfDraft(file: RecipeFile, draft: RecipeDraft): RecipeFile {
  const [base, ...joined] = draft.clips;
  return {
    ...file,
    base: base ?? file.base,
    joined,
    time: draft.time,
    mirror: draft.mirror,
    layers: draft.layers,
    blanks: draft.blanks,
  };
}

/** A bone's partner on the other side (`leftHand` and `rightHand`), or itself in the middle. */
export function otherSide(bone: string): string {
  if (bone.startsWith('left')) return `right${bone.slice(4)}`;
  if (bone.startsWith('right')) return `left${bone.slice(5)}`;
  return bone;
}

/** The changes that swap sides with the mirror, each with its bone and its partner. */
export const swapsOf = (layers: readonly RecipeLayer[]) =>
  layers.filter(l => otherSide(l.bone) !== l.bone).map(l => ({ layer: l, to: otherSide(l.bone) }));

/** The changes with each bone on its other side. */
export const swapSides = (layers: readonly RecipeLayer[]): RecipeLayer[] =>
  layers.map(l => (otherSide(l.bone) === l.bone ? l : { ...l, bone: otherSide(l.bone) }));

/** The time edits on clip `index` that sit outside `trim`, which stay in the file. */
export function outsideTrim(time: readonly TimeEdit[], index: number, trim: [number, number]) {
  const [a, b] = trim;
  return time.filter(t => {
    if (t.clip !== index) return false;
    const first = t.kind === 'retime' ? (t.from ?? 0) : (t.at ?? 0);
    const last =
      t.kind === 'retime' ? (t.to ?? 0) : t.kind === 'freeze' ? first : first + (t.frames ?? 0);
    return first < a || last > b;
  }).length;
}

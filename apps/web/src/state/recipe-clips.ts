/**
 * A recipe's clips as the editor changes them: joining a library clip,
 * taking one out, moving one, trimming and blending, and the mirror's
 * swap of sides. Each keeps the draft whole: the base has no blend, every
 * joined clip has one, `kept` says which clip of the file as read each one
 * keeps (null for a clip newly picked), and the time edits and changes
 * follow their clip's place (recipe-shares.ts moves a change's start and
 * end, so it stays on the motion it was placed on). Pure helpers; the store
 * is use-recipe-editor-store.ts.
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
import { boneWords } from './recipe-bones';
import { KIND_WORDS, type RecipeDraft } from './recipe-draft';
import { mirrorPose } from './recipe-key-edits';
import { followLayers, placesOfMove, type StrandedChange } from './recipe-shares';
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

const framesOf = (clips: readonly RecipeBase[]) => clips.map(c => c.frames);

/** A change as the clip list names it: "Reach on Right hand". */
const changeWords = (layer: RecipeLayer) =>
  layer.kind === 'pose' ? KIND_WORDS.pose : `${KIND_WORDS[layer.kind]} on ${boneWords(layer.bone)}`;

/** The draft's changes moved onto `clips`, each clip `i` of the draft now at `places[i]`. */
function follow(
  draft: RecipeDraft,
  clips: readonly RecipeBase[],
  places: readonly (number | null)[]
) {
  return followLayers(draft.layers, framesOf(draft.clips), framesOf(clips), places);
}

/** Joins `clip` after the last one; every change keeps the frames it covered. */
export function addClip(draft: RecipeDraft, clip: RecipeBase): RecipeDraft {
  const clips = blended([...draft.clips, clip]);
  return {
    ...draft,
    clips,
    kept: [...draft.kept, null],
    layers: follow(
      draft,
      clips,
      draft.clips.map((_, i) => i)
    ).layers,
  };
}

/** Why clip `index` cannot go: the time edits and changes that sit on it, if any. */
function inUseBy(draft: RecipeDraft, index: number, stranded: readonly StrandedChange[]) {
  const count = draft.time.filter(t => t.clip === index).length;
  const changes = stranded.map(s => changeWords(s.layer));
  const things = [
    ...(count === 1 ? ['a time edit'] : count > 1 ? [`${count} time edits`] : []),
    ...(changes.length === 1 ? [`the change ${changes[0]}`] : []),
    ...(changes.length > 1 ? [`the changes ${changes.join(', ')}`] : []),
  ];
  if (!things.length) return null;
  const one = things.length === 1 && (count === 1 || changes.length === 1);
  const clip = draft.clips[index];
  return `Clip ${index + 1} (${clip?.name ?? ''}) cannot go: ${things.join(' and ')} ${one ? 'sits' : 'sit'} on it. Take those out first.`;
}

/** What the core says when a clip taken out still holds time edits or changes. */
export function inUse(draft: RecipeDraft, index: number): string | null {
  const clips = draft.clips.filter((_, i) => i !== index);
  const places = draft.clips.map((_, i) => (i === index ? null : i - Number(i > index)));
  return inUseBy(draft, index, follow(draft, clips, places).stranded);
}

/** The draft without clip `index`, or why it cannot go. */
export function removeClip(
  draft: RecipeDraft,
  index: number
): { draft: RecipeDraft } | { problem: string } {
  if (draft.clips.length < 2) return { problem: 'A recipe needs a clip.' };
  const clips = blended(draft.clips.filter((_, i) => i !== index));
  const places = draft.clips.map((_, i) => (i === index ? null : i - Number(i > index)));
  const moved = follow(draft, clips, places);
  const problem = inUseBy(draft, index, moved.stranded);
  if (problem) return { problem };
  return {
    draft: {
      ...draft,
      clips,
      kept: draft.kept.filter((_, i) => i !== index),
      time: draft.time.map(t => (t.clip > index ? { ...t, clip: t.clip - 1 } : t)),
      layers: moved.layers,
    },
  };
}

/**
 * The draft with clip `from` moved to place `to`; its time edits and changes
 * go with it. A change that runs across a seam keeps its clips together, so
 * a move that would part them says why instead.
 */
export function moveClip(
  draft: RecipeDraft,
  from: number,
  to: number
): { draft: RecipeDraft } | { problem: string } {
  const n = draft.clips.length;
  if (from === to || from < 0 || to < 0 || from >= n || to >= n) return { draft };
  const places = placesOfMove(n, from, to);
  const order = places.map(() => 0);
  places.forEach((place, old) => {
    order[place] = old;
  });
  const clips = blended(order.map(i => draft.clips[i] as RecipeBase));
  const moved = follow(draft, clips, places);
  const split = moved.stranded[0];
  if (split && 'split' in split.why) {
    const i = split.why.split;
    return {
      problem: `Clip ${i + 1} (${draft.clips[i]?.name ?? ''}) cannot move away from clip ${i}: the change ${changeWords(split.layer)} runs across both. Shorten it to one clip first.`,
    };
  }
  return {
    draft: {
      ...draft,
      clips,
      kept: order.map(i => draft.kept[i] ?? null),
      time: draft.time.map(t => ({ ...t, clip: places[t.clip] ?? t.clip })),
      layers: moved.layers,
    },
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

/** The changes with each bone on its other side; a grip's pose mirrors with its hand. */
export const swapSides = (layers: readonly RecipeLayer[]): RecipeLayer[] =>
  layers.map(l => {
    if (otherSide(l.bone) === l.bone) return l;
    const swapped = { ...l, bone: otherSide(l.bone) };
    return l.kind === 'grip' && l.pose ? { ...swapped, pose: mirrorPose(l.pose) } : swapped;
  });

/** The time edits on clip `index` that sit outside `trim`, which stay in the file. */
export function outsideTrim(time: readonly TimeEdit[], index: number, trim: [number, number]) {
  const [a, b] = trim;
  return time.filter(t => {
    if (t.clip !== index) return false;
    const first = t.kind === 'retime' ? (t.from ?? 0) : (t.at ?? 0);
    const last =
      t.kind === 'retime' ? (t.to ?? 0) : t.kind === 'freeze' ? first : first + (t.frames ?? 0);
    return first < a || last > b;
  });
}

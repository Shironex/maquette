/**
 * What version 2 of the recipe file adds, in mock mode as the core has it:
 * which version a recipe is written as, the shape it is written in, the
 * checks and limits of clips, time edits, pose keys and grips, and keys
 * and time edits following their clip when the clips are replaced.
 */
import type { RecipeBase, RecipeFile, RecipeLayer, TimeEdit } from '../client';
import { DEFAULT_BLEND, V2_LIMITS, clipsProblem, timeProblem } from '@/state/recipe-v2-check';
import { fail } from './backend-options';
import { clipsOf } from './recipe-files';

export { V2_LIMITS };

/** How the core starts a refusal of clips that strand keys or time edits. */
export const CLIP_IN_USE = 'Taking out clip';

const V2_KINDS: readonly RecipeLayer['kind'][] = ['pin', 'pose', 'grip'];

/** The core's `uses_v2`: anything of version 2, down to a layer's mode or eases. */
export const usesV2 = (recipe: RecipeFile) =>
  Boolean(
    recipe.joined?.length ||
    recipe.mirror ||
    recipe.time?.length ||
    recipe.base.blend ||
    clipsOf(recipe).some(c => c.trim) ||
    recipe.layers.some(
      l =>
        V2_KINDS.includes(l.kind) ||
        l.mode ||
        l.keys?.length ||
        l.pose ||
        l.from ||
        l.settings.easeIn != null ||
        l.settings.easeOut != null ||
        l.settings.flat
    )
  );

/** A layer as the core writes it: only the fields its kind reads. */
function layerFor(l: RecipeLayer): RecipeLayer {
  const { mode, keys, pose, from, ...rest } = l;
  const { easeIn, easeOut, flat, ...settings } = l.settings;
  const eased = l.kind === 'pin' || l.kind === 'grip';
  const out: RecipeLayer = {
    ...rest,
    settings: {
      ...settings,
      ...(eased && easeIn != null ? { easeIn } : {}),
      ...(eased && easeOut != null ? { easeOut } : {}),
      ...(l.kind === 'pin' && flat ? { flat } : {}),
    },
  };
  if (l.kind === 'pose') {
    const sorted = [...(keys ?? [])].sort(
      (a, b) => (a.clip ?? 0) - (b.clip ?? 0) || a.frame - b.frame
    );
    return {
      ...out,
      bone: '',
      settings: { ...out.settings, start: 0, end: 1 },
      mode: mode ?? 'add',
      keys: sorted,
    };
  }
  if (l.kind === 'grip')
    return {
      ...out,
      mode: mode ?? 'replace',
      ...(pose ? { pose } : {}),
      ...(from ? { from } : {}),
    };
  return out;
}

/**
 * The recipe as the core writes it: version 1 while it uses nothing of
 * version 2 and 2 once it does, no blend on the base, a blend on every
 * joined clip, each layer with only what its kind reads, and the version 2
 * lists left out while empty.
 */
export function withVersion(recipe: RecipeFile): RecipeFile {
  const base = { ...recipe.base };
  delete base.blend;
  // Fields in place, so a recipe that uses nothing new reads as it did.
  const out: RecipeFile = {
    ...recipe,
    base,
    joined: recipe.joined?.map(c => ({ ...c, blend: c.blend ?? DEFAULT_BLEND })),
    layers: recipe.layers.map(layerFor),
  };
  if (!out.joined?.length) delete out.joined;
  if (!out.mirror) delete out.mirror;
  if (!out.time?.length) delete out.time;
  return { ...out, version: usesV2(out) ? 2 : 1 };
}

function turnsProblem(turns: Record<string, unknown>, most: number, what: string): string | null {
  const count = Object.keys(turns).length;
  if (count === 0) return `The ${what} turns no bone. Turn one, or take it out.`;
  if (count > most) return `The ${what} turns at most ${most} bones.`;
  for (const [bone, turn] of Object.entries(turns))
    if (!(turn as number[]).every(v => Math.abs(v) <= V2_LIMITS.turnDegrees))
      return `The ${what} turns ${bone} more than ${V2_LIMITS.turnDegrees} degrees each way.`;
  return null;
}

function layersProblem(recipe: RecipeFile): string | null {
  const clips = clipsOf(recipe);
  let keys = 0;
  let turns = 0;
  for (const layer of recipe.layers) {
    if (layer.kind === 'pose') {
      const list = layer.keys ?? [];
      if (list.length > V2_LIMITS.keys)
        return `The pose keys ${layer.id} hold at most ${V2_LIMITS.keys} keys.`;
      const seen = new Set<string>();
      for (const key of list) {
        const at = key.clip ?? 0;
        const clip = clips[at];
        if (!clip)
          return `A key of ${layer.id} sits on clip ${at + 1}, which the recipe does not have.`;
        if (key.frame >= clip.frames)
          return `A key of ${layer.id} sits on frame ${key.frame}, past the ${clip.frames} frames of clip ${at + 1}.`;
        const spot = `${at}:${key.frame}`;
        if (seen.has(spot))
          return `${layer.id} has two keys on frame ${key.frame} of clip ${at + 1}. Keep one.`;
        seen.add(spot);
        const problem = turnsProblem(
          key.pose,
          V2_LIMITS.keyBones,
          `key at frame ${key.frame} of ${layer.id}`
        );
        if (problem) return problem;
        turns += Object.keys(key.pose).length;
      }
      keys += list.length;
    }
    if (layer.kind === 'grip') {
      if (!layer.pose)
        return `The grip ${layer.id} needs its pose: degrees per hand and finger bone.`;
      const problem = turnsProblem(layer.pose, V2_LIMITS.gripBones, `grip ${layer.id}`);
      if (problem) return problem;
      turns += Object.keys(layer.pose).length;
    }
  }
  if (keys > V2_LIMITS.recipeKeys)
    return `The recipe holds ${keys} pose keys; it holds at most ${V2_LIMITS.recipeKeys} across all its pose keys. Take some out.`;
  if (turns > V2_LIMITS.recipeTurns)
    return `The recipe turns ${turns} bones across its pose keys and grips; it turns at most ${V2_LIMITS.recipeTurns}. Turn fewer bones per key or grip.`;
  return null;
}

/** The first thing of version 2 the core refuses in `recipe`, or null. */
export const v2Problem = (recipe: RecipeFile): string | null =>
  clipsProblem(clipsOf(recipe)) ??
  timeProblem(recipe.time ?? [], clipsOf(recipe)) ??
  layersProblem(recipe);

const sameClip = (a: RecipeBase, b: RecipeBase) =>
  a.name === b.name &&
  a.frames === b.frames &&
  a.hash === b.hash &&
  Boolean(a.rest) === Boolean(b.rest) &&
  Boolean(a.byName) === Boolean(b.byName);

/**
 * The core's `replace_clips`: the layers and time edits `current` keeps
 * (those not sent, `layersSent` and `timeSent` say which) moved to where
 * their clip went among `clips`, each clip found through `keptFrom` (the
 * index of the clip of `current` it keeps) or else by name, frames and
 * hash. A clip taken away that one of them sits on fails `clip-in-use`.
 */
export function followClips(
  current: RecipeFile,
  clips: readonly RecipeBase[],
  keptFrom: readonly (number | null)[] | null | undefined,
  layersSent: boolean,
  timeSent: boolean
): { layers: RecipeLayer[]; time: TimeEdit[] } {
  const old = clipsOf(current);
  if (keptFrom && keptFrom.length !== clips.length)
    fail('invalid', 'Say for every clip sent which clip it keeps, or none for a new one.');
  const taken = old.map(() => false);
  const origins = clips.map((clip, j) => {
    if (keptFrom) {
      const i = keptFrom[j];
      if (i == null) return null;
      const was = old[i];
      if (!was) fail('invalid', `The recipe has no clip ${i + 1} to keep.`);
      if (!sameClip(was, clip))
        fail(
          'invalid',
          `Clip ${j + 1} sent keeps clip ${i + 1} (${was.name}), but it is ${clip.name}. Keep a clip as it is, or send it as a new one.`
        );
      return i;
    }
    const i = old.findIndex((o, k) => !taken[k] && sameClip(o, clip));
    if (i < 0) return null;
    taken[i] = true;
    return i;
  });
  const places: (number | null)[] = old.map(() => null);
  origins.forEach((i, j) => {
    if (i != null && places[i] == null) places[i] = j;
  });
  const dropped: [number, string][] = [];
  const move = (clip: number, what: string) => {
    const to = places[clip];
    if (to == null) dropped.push([clip, what]);
    return to ?? clip;
  };
  const layers = layersSent
    ? current.layers
    : current.layers.map(l =>
        l.keys?.length
          ? {
              ...l,
              keys: l.keys.map(k => ({ ...k, clip: move(k.clip ?? 0, `the pose keys ${l.id}`) })),
            }
          : l
      );
  const time = timeSent
    ? (current.time ?? [])
    : (current.time ?? []).map(t => ({ ...t, clip: move(t.clip, 'a time edit') }));
  const first = dropped[0];
  if (first) {
    const [clip] = first;
    const what = [...new Set(dropped.filter(([c]) => c === clip).map(([, w]) => w))];
    fail(
      'clip-in-use',
      `${CLIP_IN_USE} ${clip + 1} (${old[clip]?.name ?? ''}) would leave ${what.join(', ')} on no clip. Keep that clip (keep ${clip}), or send changes and time without what sits on it.`
    );
  }
  return { layers, time };
}

/** The biggest recipe file the core reads or writes, bytes. */
export const MAX_FILE_BYTES = 256 * 1024;

/**
 * The core's size check before a write: a recipe whose file would be over
 * {@link MAX_FILE_BYTES} fails `recipe-too-big`, since it would not open again.
 */
export function checkSize(recipe: RecipeFile) {
  const bytes = new TextEncoder().encode(`${JSON.stringify(recipe, null, 2)}\n`).length;
  if (bytes > MAX_FILE_BYTES)
    fail(
      'recipe-too-big',
      `The recipe would be over ${MAX_FILE_BYTES / 1024} KB, too big to save. Take out some pose keys, grips or changes.`
    );
}

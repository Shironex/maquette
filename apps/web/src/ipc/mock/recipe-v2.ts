/**
 * What version 2 of the recipe file adds, in mock mode as the core has it:
 * which version a recipe is written as, the shape it is written in, the
 * checks and limits of clips, time edits, pose keys and grips, and keys
 * and time edits following their clip when the clips are replaced.
 */
import type { BlendWindow, RecipeBase, RecipeFile, RecipeLayer, TimeEdit } from '../client';
import { fail } from './backend-options';
import { clipsOf } from './recipe-files';

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
} as const;

/** How the core starts a refusal of clips that strand keys or time edits. */
export const CLIP_IN_USE = 'Taking out clip';

const DEFAULT_BLEND: BlendWindow = { frames: 8, curve: 'smooth', lineUp: 'hips-and-facing' };

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

/** How many frames of `clip` its trim keeps. */
const keptFrames = (clip: RecipeBase) => (clip.trim ? clip.trim[1] - clip.trim[0] : clip.frames);

function clipsProblem(recipe: RecipeFile): string | null {
  const clips = clipsOf(recipe);
  if (clips.length > V2_LIMITS.clips) return `A recipe joins at most ${V2_LIMITS.clips} clips.`;
  if (recipe.base.rest && clips.length > 1)
    return 'A recipe on the rest pose has no clips to join. Start it on a clip instead.';
  for (const [i, clip] of clips.entries()) {
    if (i > 0 && clip.rest)
      return `Only the base can be the rest pose, and then alone; clip ${i + 1} cannot.`;
    if (!clip.trim) continue;
    if (clip.rest) return 'The rest pose has no clip to trim. Set how many frames it runs instead.';
    const [from, to] = clip.trim;
    if (from >= to || to > clip.frames)
      return `The clip ${i + 1} keeps frames ${from} to ${to}; keep at least one frame within its ${clip.frames} frames, the first before the last.`;
  }
  for (const [i, clip] of clips.entries()) {
    const before = clips[i - 1];
    if (!before) continue;
    const most = Math.floor(Math.min(keptFrames(before), keptFrames(clip)) / 2);
    const frames = clip.blend?.frames ?? DEFAULT_BLEND.frames;
    if (frames > most)
      return `The blend into clip ${i + 1} lasts ${frames} frames; it can last at most ${most}, half the shorter clip as trimmed.`;
  }
  return null;
}

function timeProblem(recipe: RecipeFile): string | null {
  const time = recipe.time ?? [];
  if (time.length > V2_LIMITS.timeEdits)
    return `A recipe holds at most ${V2_LIMITS.timeEdits} time edits.`;
  const clips = clipsOf(recipe);
  const spans: [TimeEdit, number, number][] = [];
  for (const edit of time) {
    const clip = clips[edit.clip];
    if (!clip) return `A time edit sits on clip ${edit.clip + 1}, which the recipe does not have.`;
    const where = `clip ${edit.clip + 1}`;
    if (edit.kind === 'retime') {
      const { from, to, speed } = edit;
      if (from == null || to == null || speed == null)
        return `A retime on ${where} needs from, to and speed.`;
      if (from >= to || to > clip.frames)
        return `A retime on ${where} runs from frame ${from} to ${to}; keep it within its ${clip.frames} frames, from before to.`;
      if (speed < 0.25 || speed > 4) return `A retime plays 0.25x to 4x, not ${speed}x.`;
      spans.push([edit, from, to]);
    } else {
      const word = edit.kind === 'freeze' ? 'freeze' : 'hold over';
      const { at, frames } = edit;
      if (at == null || frames == null) return `A ${word} on ${where} needs at and frames.`;
      if (at >= clip.frames)
        return `A ${word} on ${where} holds frame ${at}, past its ${clip.frames} frames.`;
      if (frames < 1 || frames > V2_LIMITS.holdFrames)
        return `A ${word} lasts 1 to ${V2_LIMITS.holdFrames} frames, not ${frames}.`;
      if (edit.kind === 'hold-over' && at + frames > clip.frames)
        return `A hold over on ${where} keeps the length, so it has to end within its ${clip.frames} frames.`;
      spans.push([edit, at, edit.kind === 'hold-over' ? at + frames : at + 1]);
    }
  }
  for (const [i, [edit, first, end]] of spans.entries()) {
    const clash = spans
      .slice(0, i)
      .some(
        ([o, a, b]) =>
          o.clip === edit.clip &&
          (o.kind === 'retime') === (edit.kind === 'retime') &&
          a < end &&
          first < b
      );
    if (clash)
      return edit.kind === 'retime'
        ? `Two retime ranges on clip ${edit.clip + 1} overlap. Keep each range apart.`
        : `Two holds on clip ${edit.clip + 1} overlap. Keep each hold apart.`;
  }
  return null;
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
  clipsProblem(recipe) ?? timeProblem(recipe) ?? layersProblem(recipe);

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
      if (i != null && !old[i]) fail('invalid', `The recipe has no clip ${i + 1} to keep.`);
      return i ?? null;
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

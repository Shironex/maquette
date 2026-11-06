/**
 * Recipe files in mock mode, as the core reads them: a stand-in for the
 * hash of the bytes, the base clip a recipe keeps of a library clip, and
 * the match of that base in the clip library by name and frame count.
 * Shared by the project recipes and the recipe library.
 */
import type { AnimationClip, BaseMatch, RecipeBase, RecipeFile, RecipeLayer } from '../client';
import { clock } from '@/lib/clock';

/** A stand-in for the core's hash of a file: 16 hex digits that change with any content. */
export function versionOf(value: unknown): string {
  const text = JSON.stringify(value);
  let a = 0x811c9dc5;
  let b = 0x01000193;
  for (let i = 0; i < text.length; i += 1) {
    a = Math.imul(a ^ text.charCodeAt(i), 0x01000193);
    b = Math.imul(b ^ text.charCodeAt(i), 0x811c9dc5);
  }
  const hex = (n: number) => (n >>> 0).toString(16).padStart(8, '0');
  return hex(a) + hex(b);
}

export const baseOf = (clip: AnimationClip): RecipeBase => ({
  name: clip.name,
  frames: clip.frames,
  length: clip.length ?? 0,
  hash: clip.hash,
  mixamo: clip.rig.kind === 'mixamo',
});

/** The rest pose's frame rate and how long a recipe on it runs unless told, as the core has them. */
export const REST_FPS = 30;
export const REST_FRAMES = 60;

/** The rest pose held for `frames` frames, no base clip. */
export const restBase = (frames = REST_FRAMES): RecipeBase => ({
  name: 'Rest pose',
  frames,
  length: frames / REST_FPS,
  hash: '',
  mixamo: false,
  rest: true,
});

/** A base known by its clip's name only (an empty hash), as a starter names it. */
export const namedBase = (name: string): RecipeBase => ({
  name,
  frames: 1,
  length: 1 / REST_FPS,
  hash: '',
  mixamo: false,
});

/** The first clip called `name`, case aside, whatever its frames. */
export const clipNamed = (clips: readonly AnimationClip[], name: string) =>
  clips.find(c => c.name.trim().toLowerCase() === name.trim().toLowerCase());

export function matchBase(
  clips: readonly AnimationClip[],
  base: RecipeBase
): [BaseMatch, AnimationClip?] {
  if (base.rest) return [{ status: 'rest', clip: null, warning: null }];
  if (!base.hash) {
    const found = clipNamed(clips, base.name);
    if (found) return [{ status: 'found', clip: found.id, warning: null }, found];
    return [
      {
        status: 'missing',
        clip: null,
        warning: `The animation library has no clip called ${base.name}. Import one before baking.`,
      },
    ];
  }
  const name = base.name.trim().toLowerCase();
  const same = clips.filter(c => c.frames === base.frames && c.name.trim().toLowerCase() === name);
  const exact = same.find(c => c.hash === base.hash);
  if (exact) return [{ status: 'found', clip: exact.id, warning: null }, exact];
  const other = same[0];
  if (other)
    return [
      {
        status: 'hash-differs',
        clip: other.id,
        warning: `The library's ${other.name} has other keys than the recipe was made on (its hash differs). A bake will use the library's.`,
      },
      other,
    ];
  return [
    {
      status: 'missing',
      clip: null,
      warning: `The animation library has no clip called ${base.name} with ${base.frames} frames. Import it before baking.`,
    },
  ];
}

/** A bake of a recipe on `base` counts as Mixamo's: it says so, or its library clip came from Mixamo. */
export const isMixamo = (clips: readonly AnimationClip[], base: RecipeBase) =>
  base.mixamo || matchBase(clips, base)[1]?.rig.kind === 'mixamo';

export function newRecipe(name: string, base: RecipeBase): RecipeFile {
  const now = clock.now().toISOString();
  return { version: 1, name, revision: 1, created: now, edited: now, base, blanks: [], layers: [] };
}

export const layer = (
  id: string,
  kind: RecipeLayer['kind'],
  bone: string,
  settings: Partial<RecipeLayer['settings']>
): RecipeLayer => ({
  id,
  kind,
  bone,
  settings: {
    weight: 1,
    start: 0,
    end: 1,
    target: null,
    rotation: null,
    offset: null,
    cycles: null,
    amount: null,
    ...settings,
  },
});

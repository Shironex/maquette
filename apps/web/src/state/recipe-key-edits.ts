/**
 * Edits to a pose key change's keys, as the editor makes them: turn a bone
 * at a frame, change a key's ease, move or copy a key, remove a bone or a
 * whole key, and mirror a pose. The same rules as the core's `edit_keys`
 * (crates/maquette-model/src/recipes/key_edits.rs), which agents use: keys
 * stay in clip and frame order, a key with no bones left goes, and bones
 * moved onto a key that is there keep that key's ease.
 */
import type { KeyEase, LayerMode, PoseKey, RecipeLayer } from '@/ipc/client';
import type { Turn } from './recipe-keys';

/** Where a key sits: a clip (0 the base) and that clip's own frame. */
export interface KeySpot {
  clip: number;
  frame: number;
}

type Pose = PoseKey['pose'];

export const sameSpot = (k: Pick<PoseKey, 'clip' | 'frame'>, at: KeySpot) =>
  (k.clip ?? 0) === at.clip && k.frame === at.frame;

const order = (a: PoseKey, b: PoseKey) => (a.clip ?? 0) - (b.clip ?? 0) || a.frame - b.frame;

/** `keys` without empty keys, in clip and frame order. */
const tidy = (keys: PoseKey[]): PoseKey[] =>
  keys.filter(k => Object.keys(k.pose).length > 0).sort(order);

const withKeys = (layer: RecipeLayer, keys: PoseKey[]): RecipeLayer => ({
  ...layer,
  keys: tidy(keys),
});

/** The key at `at`, if there is one. */
export const keyAt = (layer: Pick<RecipeLayer, 'keys'>, at: KeySpot): PoseKey | null =>
  (layer.keys ?? []).find(k => sameSpot(k, at)) ?? null;

/** Puts `pose` into the key at `at`, making it with `ease` when there is none. */
export function setPose(
  layer: RecipeLayer,
  at: KeySpot,
  pose: Pose,
  ease: KeyEase = 'smooth'
): RecipeLayer {
  const keys = layer.keys ?? [];
  const there = keyAt(layer, at);
  if (!there) return withKeys(layer, [...keys, { clip: at.clip, frame: at.frame, ease, pose }]);
  return withKeys(
    layer,
    keys.map(k => (k === there ? { ...k, pose: { ...k.pose, ...pose } } : k))
  );
}

/** Turns `bone` to `turn` at `at`, keying it there. */
export const setTurn = (layer: RecipeLayer, at: KeySpot, bone: string, turn: Turn) =>
  setPose(layer, at, { [bone]: turn });

/** The key at `at` with its ease set to `ease`. */
export const setEase = (layer: RecipeLayer, at: KeySpot, ease: KeyEase): RecipeLayer =>
  withKeys(
    layer,
    (layer.keys ?? []).map(k => (sameSpot(k, at) ? { ...k, ease } : k))
  );

/** Removes `bone` from the key at `at`, or the whole key when `bone` is null. */
export function removeKey(layer: RecipeLayer, at: KeySpot, bone: string | null): RecipeLayer {
  return withKeys(
    layer,
    (layer.keys ?? []).flatMap(k => {
      if (!sameSpot(k, at)) return [k];
      if (bone === null) return [];
      const { [bone]: _gone, ...pose } = k.pose;
      return [{ ...k, pose }];
    })
  );
}

/**
 * Moves (or with `copy`, copies) the key at `from` to `to`: only `bones`
 * when given, else the whole key. Onto a key that is there, the bones join
 * it and its ease stays.
 */
export function moveKey(
  layer: RecipeLayer,
  from: KeySpot,
  to: KeySpot,
  { copy = false, bones }: { copy?: boolean; bones?: readonly string[] } = {}
): RecipeLayer {
  const key = keyAt(layer, from);
  if (!key || sameSpot(key, to)) return layer;
  const moved = Object.fromEntries(
    Object.entries(key.pose).filter(([b]) => !bones || bones.includes(b))
  ) as Pose;
  let next = layer;
  if (!copy) {
    next = withKeys(
      next,
      (next.keys ?? []).map(k =>
        k === key
          ? {
              ...k,
              pose: Object.fromEntries(
                Object.entries(k.pose).filter(([b]) => !(b in moved))
              ) as Pose,
            }
          : k
      )
    );
  }
  return setPose(next, to, moved, key.ease ?? 'smooth');
}

/** The other side's bone: `leftHand` and `rightHand` swap, the middle stays. */
export const otherSide = (bone: string) =>
  bone.startsWith('left')
    ? `right${bone.slice(4)}`
    : bone.startsWith('right')
      ? `left${bone.slice(5)}`
      : bone;

const flip = (v: number | null) => (v === null || v === 0 ? v : -v);

/**
 * `pose` as its mirror image across the body's middle: each side's bones
 * take the other's turns, X kept and Y and Z flipped, as a mirror shows a
 * turn about the model's own axes.
 */
export function mirrorPose(pose: Pose): Pose {
  return Object.fromEntries(
    Object.entries(pose).map(([bone, [x, y, z]]) => [otherSide(bone), [x, flip(y), flip(z)]])
  ) as Pose;
}

/** A new, empty pose key change: no keys yet, laid on as `mode`. */
export function emptyKeys(id: string, mode: LayerMode = 'add'): RecipeLayer {
  return {
    id,
    kind: 'pose',
    bone: '',
    mode,
    keys: [],
    settings: {
      weight: 1,
      start: 0,
      end: 1,
      target: null,
      rotation: null,
      offset: null,
      cycles: null,
      amount: null,
    },
  };
}

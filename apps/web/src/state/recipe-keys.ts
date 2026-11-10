/**
 * Pose keys as they play: where each key stands on the recipe's timeline
 * and the turn each keyed bone has between keys. A copy of the core's rules
 * (crates/maquette-model/src/recipes/pose_keys.rs), which the bake follows;
 * fixtures/pose_keys.json there holds the cases both agree on.
 *
 * A key sits on a clip's own frame, so it stands at the share of the clips'
 * own frames laid end to end, as round 1's changes do: a trim, retime,
 * freeze or blend carries it with its motion. Each bone blends only between
 * the keys that turn it; before its first key it holds that key, after its
 * last the last, and between two it moves as the later key's ease says.
 */
import type { KeyEase, LayerMode, PoseKey, RecipeFile, RecipeLayer } from '@/ipc/client';
import { num } from './recipe-draft';
import { frameOfClip } from './recipe-time';
import { clipsOf, plain, type Playable, type Timeline } from './recipe-timeline';

/** Degrees about the model's own axes (X to its left, Y up, Z ahead), X then Y then Z. */
export type Turn = [number, number, number];

const EPS = 1e-9;

export const EASES: readonly KeyEase[] = ['smooth', 'in', 'out', 'linear', 'step'];

export const EASE_WORDS: Record<KeyEase, string> = {
  smooth: 'Smooth',
  in: 'In',
  out: 'Out',
  linear: 'Linear',
  step: 'Step',
};

export const EASE_ABOUT: Record<KeyEase, string> = {
  smooth: 'Slow out of the key before and slow into this one',
  in: 'Slow out of the key before, fast into this one',
  out: 'Fast out of the key before, slow into this one',
  linear: 'An even pace',
  step: 'Holds the key before until this frame, then jumps',
};

/** How far along `t` (0 to 1, from the key before to this one) the motion is. */
export function ease(kind: KeyEase, t: number): number {
  const x = Math.min(Math.max(t, 0), 1);
  switch (kind) {
    case 'in':
      return x * x;
    case 'out':
      return 1 - (1 - x) * (1 - x);
    case 'linear':
      return x;
    case 'step':
      return x >= 1 - EPS ? 1 : 0;
    case 'smooth':
      return x * x * (3 - 2 * x);
  }
}

/** A key's turn on `bone` as numbers, a null read as 0. */
export const turnOf = (key: PoseKey, bone: string): Turn | null => {
  const t = key.pose[bone];
  return t ? [num(t[0]), num(t[1]), num(t[2])] : null;
};

/** Where a key on frame `frame` of clip `clip` stands: a share of the clips' frames end to end. */
export function keyShare(recipe: Playable, clip: number, frame: number): number {
  const clips = clipsOf(recipe);
  const all = clips.reduce((sum, c) => sum + c.frames, 0);
  if (all <= 0) return 0;
  const before = clips.slice(0, clip).reduce((sum, c) => sum + c.frames, 0);
  return Math.min(Math.max((before + frame) / all, 0), 1);
}

/** Every bone `layer` keys, in name order. */
export const keyedBones = (layer: Pick<RecipeLayer, 'keys'>): string[] =>
  [...new Set((layer.keys ?? []).flatMap(k => Object.keys(k.pose)))].sort();

type Track = { share: number; ease: KeyEase; turn: Turn }[];

function sample(track: Track, share: number): Turn | null {
  const first = track[0];
  if (!first) return null;
  let i = -1;
  track.forEach((p, k) => {
    if (p.share <= share + EPS) i = k;
  });
  if (i < 0) return first.turn;
  const a = track[i]!;
  const b = track[i + 1];
  if (!b) return a.turn;
  const span = b.share - a.share;
  const e = ease(b.ease, span > EPS ? (share - a.share) / span : 1);
  return [0, 1, 2].map(k => a.turn[k]! + (b.turn[k]! - a.turn[k]!) * e) as Turn;
}

/** The turn each bone `layer` keys has at `share`, by bone name. */
export function layerTurns(recipe: Playable, layer: RecipeLayer, share: number): Map<string, Turn> {
  const keys = (layer.keys ?? [])
    .map(k => ({ share: keyShare(recipe, k.clip ?? 0, k.frame), key: k }))
    .sort((a, b) => a.share - b.share);
  const out = new Map<string, Turn>();
  for (const bone of keyedBones(layer)) {
    const track: Track = keys.flatMap(({ share: s, key }) => {
      const turn = turnOf(key, bone);
      return turn ? [{ share: s, ease: key.ease ?? 'smooth', turn }] : [];
    });
    const turn = sample(track, share);
    if (turn) out.set(bone, turn);
  }
  return out;
}

/** The share bake frame `frame` stands on: frame over count on one whole clip, else the timeline's. */
export function shareAt(recipe: RecipeFile, t: Timeline, frame: number): number {
  if (plain(recipe)) {
    const frames = recipe.base.frames;
    return frames > 0 ? Math.min(Math.max(frame / frames, 0), 1) : 0;
  }
  return t.keys[Math.min(Math.max(Math.round(frame), 0), t.frames)]?.share ?? 0;
}

/** The clip and its own frame bake frame `frame` shows: in a blend, the clip that shows more. */
export function clipFrameAt(t: Timeline, frame: number): { clip: number; frame: number } {
  const key = t.keys[Math.min(Math.max(Math.round(frame), 0), t.frames)];
  if (!key) return { clip: 0, frame: Math.max(Math.round(frame), 0) };
  const side = key.to && key.blend >= 0.5 ? key.to : key.from;
  return { clip: side.clip, frame: Math.max(Math.round(side.at), 0) };
}

/** The bake frame a key shows on, or null when its clip's trim cuts it out. */
export const keyFrame = (t: Timeline, clip: number, frame: number): number | null =>
  frameOfClip(t, clip, frame);

/** A bone's turn from one pose key change at one frame. */
export interface KeyedTurn {
  layer: string;
  bone: string;
  mode: LayerMode;
  weight: number;
  turn: Turn;
}

/** Every pose key change's turns at bake frame `frame`, in the order they are laid on. */
export function poseAt(recipe: RecipeFile, t: Timeline, frame: number): KeyedTurn[] {
  const share = shareAt(recipe, t, frame);
  return recipe.layers.flatMap(layer => {
    if (layer.kind !== 'pose') return [];
    const mode = layer.mode ?? 'add';
    const weight = Math.min(Math.max(num(layer.settings.weight, 1), 0), 1);
    return [...layerTurns(recipe, layer, share)].map(([bone, turn]) => ({
      layer: layer.id,
      bone,
      mode,
      weight,
      turn,
    }));
  });
}

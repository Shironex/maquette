/**
 * A change's start and end follow its clips. They are shares of the clips'
 * own frames laid end to end (recipe-timeline.ts), so joining, taking out or
 * moving a clip moves every share: each is read back to its clip and frame
 * and written again where that clip now sits, and a change across a seam
 * keeps the clips it spans together, in order. A copy of the core's rules
 * (crates/maquette-model/src/recipes/clip_moves.rs); fixtures/clip_moves.json
 * there holds the cases both agree on. Pure helpers, used by the editor's
 * clip edits, the merge of an outside write and the mock backend.
 */
import type { RecipeLayer } from '@/ipc/client';

/** Why a change cannot follow its clips: the clip it sits on goes, or the clips it spans part. */
export type Stranded = { dropped: number } | { split: number };

/**
 * The clip frame `x` of the clips laid end to end sits on, and that clip's
 * own frame. A start on a seam belongs to the clip after it, an end to the
 * clip before.
 */
function locate(frames: readonly number[], x: number, end: boolean): [number, number] | null {
  let before = 0;
  let last: [number, number] | null = null;
  for (let i = 0; i < frames.length; i += 1) {
    const n = frames[i] ?? 0;
    if (n > 0) {
      if (end ? x <= before + n : x < before + n) return [i, Math.min(Math.max(x - before, 0), n)];
      last = [i, n];
    }
    before += n;
  }
  return last;
}

/** A share as written: to a millionth, as the core rounds it. */
const rounded = (v: number) => Math.round(Math.min(Math.max(v, 0), 1) * 1e6) / 1e6;

const sum = (v: readonly number[]) => v.reduce((a, b) => a + b, 0);

/**
 * A change's `start` and `end` on clips of `from` frames, moved to where
 * their clips went among clips of `to` frames (`places[i]` is clip `i`'s new
 * place, null when it goes). A share whose clip keeps its place and whose
 * clips keep their length comes back as it was.
 */
export function moveShare(
  from: readonly number[],
  to: readonly number[],
  places: readonly (number | null)[],
  start: number,
  end: number
): { start: number; end: number } | Stranded {
  const allOld = sum(from);
  const allNew = sum(to);
  if (allOld <= 0 || allNew <= 0) return { start, end };
  const a = locate(from, start * allOld, false);
  const b = locate(from, end * allOld, true);
  if (!a || !b) return { start, end };
  const place = (i: number) => places[i] ?? null;
  for (let i = a[0]; i <= b[0]; i += 1) if (place(i) == null) return { dropped: i };
  for (let i = a[0] + 1; i <= b[0]; i += 1) {
    const before = place(i - 1);
    if (before == null || place(i) !== before + 1) return { split: i };
  }
  const moved = ([i, at]: [number, number], share: number) => {
    const was = sum(from.slice(0, i));
    const now = sum(to.slice(0, place(i) ?? i));
    return was === now && allOld === allNew ? share : rounded((now + at) / allNew);
  };
  return { start: moved(a, start), end: moved(b, end) };
}

/** A change that cannot follow its clips, and why. */
export interface StrandedChange {
  layer: RecipeLayer;
  why: Stranded;
}

/**
 * `layers` with each change's start and end moved to where its clips went,
 * and its pose keys moved to their clip's new place. Pose key changes always
 * cover the whole recipe, so only their keys move. The changes that cannot
 * follow come back in `stranded`, unmoved in `layers`.
 */
export function followLayers(
  layers: readonly RecipeLayer[],
  from: readonly number[],
  to: readonly number[],
  places: readonly (number | null)[]
): { layers: RecipeLayer[]; stranded: StrandedChange[] } {
  const stranded: StrandedChange[] = [];
  const out = layers.map(layer => {
    let next = layer;
    if (layer.kind !== 'pose') {
      const { start, end } = layer.settings;
      const moved = moveShare(from, to, places, start ?? 0, end ?? 1);
      if ('dropped' in moved || 'split' in moved) stranded.push({ layer, why: moved });
      else if (moved.start !== layer.settings.start || moved.end !== layer.settings.end)
        next = { ...layer, settings: { ...layer.settings, ...moved } };
    }
    if (layer.keys?.length) {
      const gone = layer.keys.find(k => places[k.clip ?? 0] == null);
      if (gone) stranded.push({ layer, why: { dropped: gone.clip ?? 0 } });
      else
        next = {
          ...next,
          keys: layer.keys.map(k => ({ ...k, clip: places[k.clip ?? 0] ?? k.clip ?? 0 })),
        };
    }
    return next;
  });
  return { layers: out, stranded };
}

/** Each clip's new place after clip `from` moves to place `to` among `n` clips. */
export function placesOfMove(n: number, from: number, to: number): number[] {
  const order = Array.from({ length: n }, (_, i) => i);
  order.splice(to, 0, ...order.splice(from, 1));
  const places = order.map(() => 0);
  order.forEach((old, place) => {
    places[old] = place;
  });
  return places;
}

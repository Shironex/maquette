/**
 * The mock bake's joint limits check of a recipe's pose keys, as the core
 * judges them (keyed_limits in recipes/joint_limits.rs, joints_line in
 * bake/review.rs): each keyed turn against the humanoid table, the furthest
 * past first, and the review line it makes.
 */
import type {
  KeyedLimits,
  RecipeBakeResult,
  RecipeDoc,
  RecipeReviewLine,
} from '@/ipc/generated/bindings';
import { overLimits } from '@/state/joint-limits';
import { keyFrame, type Turn } from '@/state/recipe-keys';
import { plain, timeline } from '@/state/recipe-timeline';

/** Every keyed turn of `doc` judged, or none for a recipe with no pose keys. */
export function jointsOf(doc: RecipeDoc): KeyedLimits | null {
  const recipe = doc.recipe;
  const pose = recipe.layers.filter(l => l.kind === 'pose');
  if (!pose.length) return null;
  const t = timeline(recipe);
  const whole = plain(recipe);
  const out: KeyedLimits = { judged: 0, hits: [] };
  for (const layer of pose)
    for (const key of layer.keys ?? []) {
      const clip = key.clip ?? 0;
      const bakeFrame = whole
        ? key.frame <= t.frames
          ? key.frame
          : null
        : keyFrame(t, clip, key.frame);
      for (const [bone, turn] of Object.entries(key.pose)) {
        out.judged += 1;
        for (const h of overLimits(bone, turn.map(v => v ?? 0) as Turn))
          out.hits.push({
            layer: layer.id,
            clip,
            frame: key.frame,
            bakeFrame,
            hit: { bone, axis: h.axis, word: h.word, degrees: h.degrees, limit: h.limit },
          });
      }
    }
  const over = (h: KeyedLimits['hits'][number]) =>
    Math.abs((h.hit.degrees ?? 0) - (h.hit.limit ?? 0));
  out.hits.sort((a, b) => over(b) - over(a));
  return out;
}

/** The review's joint limits line for `result`, or null when it has no pose keys. */
function jointsLine(result: RecipeBakeResult): RecipeReviewLine | null {
  const joints = result.joints;
  if (!joints) return null;
  const worst = joints.hits[0];
  const base = { id: 'joints', label: 'Joint limits' };
  if (!worst)
    return {
      ...base,
      about: null,
      value: 'inside',
      limit: 'the humanoid table',
      measured: null,
      max: null,
      unit: null,
      status: 'pass',
      note: `${joints.judged} keyed turns, all inside. Keyed poses only.`,
    };
  const { hit } = worst;
  const degrees = Math.abs(hit.degrees ?? 0);
  const limit = Math.abs(hit.limit ?? 0);
  const at =
    worst.bakeFrame === null
      ? `frame ${worst.frame} of clip ${worst.clip + 1}, cut out by its trim`
      : `frame ${worst.bakeFrame}`;
  const more = joints.hits.length - 1;
  return {
    ...base,
    about: `${hit.bone} (${worst.layer})`,
    value: `${hit.bone} ${hit.word} ${degrees.toFixed(0)} deg`,
    limit: `${limit.toFixed(0)} deg`,
    measured: Math.round(degrees * 10) / 10,
    max: limit,
    unit: 'deg',
    status: 'warn',
    note: `The key at ${at} turns past the humanoid limit${more > 0 ? `, and ${more} more` : ''}. Keyed poses only; it can still be added.`,
  };
}

/** The review's joint limits line for `result`, none when it has no pose keys. */
export const jointLines = (result: RecipeBakeResult): RecipeReviewLine[] => {
  const line = jointsLine(result);
  return line ? [line] : [];
};

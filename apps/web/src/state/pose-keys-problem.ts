/**
 * The core's checks of a pose key change (check_pose in
 * crates/maquette-model/src/recipes/check_edits.rs), in its words, so the
 * pose keys form says what to fix before Save: at most 64 keys, each on a
 * clip the recipe has and a frame within it, one key per clip and frame,
 * each turning at most 32 humanoid bones within 180 degrees.
 */
import type { RecipeBase, RecipeLayer } from '@/ipc/client';
import { HUMAN_BONES } from './recipe-bones';
import { num } from './recipe-draft';
import { V2_LIMITS } from './recipe-v2-check';

const clipWord = (i: number) => (i === 0 ? 'the base clip' : `clip ${i + 1}`);

/** What the core would refuse in pose key change `layer` on `clips`, or null. */
export function poseKeysProblem(layer: RecipeLayer, clips: readonly RecipeBase[]): string | null {
  const { id } = layer;
  const weight = num(layer.settings.weight, Number.NaN);
  if (!(weight >= 0 && weight <= 1)) return 'The weight has to be 0 to 1.';
  const keys = layer.keys ?? [];
  if (keys.length > V2_LIMITS.keys)
    return `The pose keys ${id} hold at most ${V2_LIMITS.keys} keys.`;
  const seen = new Set<string>();
  for (const key of keys) {
    const at = key.clip ?? 0;
    const clip = clips[at];
    if (!clip) return `A key of ${id} sits on clip ${at + 1}, which the recipe does not have.`;
    if (key.frame >= clip.frames)
      return `A key of ${id} sits on frame ${key.frame}, past the ${clip.frames} frames of ${clipWord(at)}.`;
    const spot = `${at}:${key.frame}`;
    if (seen.has(spot))
      return `${id} has two keys on frame ${key.frame} of ${clipWord(at)}. Keep one.`;
    seen.add(spot);
    const what = `key at frame ${key.frame} of ${id}`;
    const bones = Object.entries(key.pose);
    if (!bones.length) return `The ${what} turns no bone. Turn one, or take it out.`;
    if (bones.length > V2_LIMITS.keyBones)
      return `The ${what} turns at most ${V2_LIMITS.keyBones} bones.`;
    for (const [bone, turn] of bones) {
      if (!HUMAN_BONES.includes(bone))
        return `The ${what} turns "${bone}", which is not a humanoid bone.`;
      const ok = turn.every(
        v => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= V2_LIMITS.turnDegrees
      );
      if (!ok)
        return `The ${what} turns ${bone} more than ${V2_LIMITS.turnDegrees} degrees each way.`;
    }
  }
  return null;
}

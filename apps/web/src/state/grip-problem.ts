/**
 * The core's checks of a grip change (check_grip in
 * crates/maquette-model/src/recipes/check_edits.rs), in its words, so the
 * grip form says what to fix before Save: the blank it holds is one the
 * recipe has, it holds a pose of at most 64 humanoid bones each turned
 * within 180 degrees, and it eases over whole frames.
 */
import type { RecipeBlank, RecipeLayer } from '@/ipc/client';
import { HUMAN_BONES } from './recipe-bones';
import { V2_LIMITS } from './recipe-v2-check';

/** The most frames a grip eases over, as the core caps it. */
export const MAX_GRIP_EASE = 100_000;

/** What the core would refuse in grip change `layer`, or null. */
export function gripProblem(layer: RecipeLayer, blanks: readonly RecipeBlank[]): string | null {
  const { id, settings: s, pose } = layer;
  if (s.target && !blanks.some(b => b.id === s.target))
    return `It holds the blank ${s.target}, which the recipe does not have.`;
  if (!pose) return `The grip ${id} needs its pose: degrees per hand and finger bone.`;
  const bones = Object.entries(pose);
  if (!bones.length) return `The grip ${id} turns no bone. Turn one, or take it out.`;
  if (bones.length > V2_LIMITS.gripBones)
    return `The grip ${id} turns at most ${V2_LIMITS.gripBones} bones.`;
  for (const [bone, turn] of bones) {
    if (!HUMAN_BONES.includes(bone)) return `The grip ${id} turns ${bone}, not a humanoid bone.`;
    const within = turn.every(
      v => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= V2_LIMITS.turnDegrees
    );
    if (!within)
      return `The grip ${id} turns ${bone} more than ${V2_LIMITS.turnDegrees} degrees each way.`;
  }
  for (const ease of [s.easeIn, s.easeOut]) {
    if (ease == null) continue;
    if (!Number.isInteger(ease) || ease < 0 || ease > MAX_GRIP_EASE)
      return `Eases are whole frames, 0 to ${MAX_GRIP_EASE}.`;
  }
  return null;
}

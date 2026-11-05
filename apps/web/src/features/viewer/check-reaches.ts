/**
 * Cheap live checks on a recipe's rough preview, measured at each reach's
 * end: how far the hand or foot stays from its target, and whether the
 * elbow or knee bends the other way than the clip has it there. Slice 4's
 * review measures the real bake; these only warn early.
 */
import type * as THREE from 'three';
import type { RecipeLayer } from '@/ipc/client';
import type { ReachCheck } from '@/state/recipe-review';
import { bendOf, poseRecipe, worldPosition, type RecipeRig } from './recipe-pose';

const num = (v: number | null | undefined, or = 0) =>
  typeof v === 'number' && Number.isFinite(v) ? v : or;

/**
 * Poses the clip at each reach's end with `frameAt` (a share of the clip),
 * lays every change over it and measures. The caller poses the frame on
 * show again afterwards.
 */
export function checkReaches(
  frameAt: (share: number) => void,
  rig: RecipeRig,
  layers: readonly RecipeLayer[],
  targets: ReadonlyMap<string, THREE.Vector3>
): ReachCheck[] {
  const out: ReachCheck[] = [];
  for (const layer of layers) {
    if (layer.kind !== 'reach') continue;
    const target = layer.settings.target ? targets.get(layer.settings.target) : undefined;
    const end = rig.bone(layer.bone);
    const limb = rig.limb(layer.bone);
    if (!target || !end || !limb) continue;
    const share = Math.min(num(layer.settings.end, 1), 0.9999);
    frameAt(share);
    const bend = () =>
      bendOf(worldPosition(limb.upper), worldPosition(limb.lower), worldPosition(end));
    const before = bend();
    poseRecipe(rig, layers, targets, share);
    const after = bend();
    const flipped = before.lengthSq() > 1e-10 && after.lengthSq() > 1e-10 && before.dot(after) < 0;
    out.push({ layer: layer.id, gap: worldPosition(end).distanceTo(target), flipped });
  }
  return out;
}

/**
 * Pose keys laid on the rough preview, with the bake's math
 * (crates/maquette-core/src/animations/recipe_keys.py). A key's turn is
 * degrees about the model's own axes (X to its left, Y up, Z ahead), X then
 * Y then Z, as the bone stands under its parent: with the parent's turn
 * from its rest P and B the model's axes in the world, add turns the bone
 * by P (B K B^-1) P^-1 on top of the clip, and replace sets its world
 * rotation to P (B K B^-1) R, R its rest. Parents go before their children.
 */
import * as THREE from 'three';
import type { LayerMode } from '@/ipc/client';
import type { Turn } from '@/state/recipe-keys';
import { modelBasis } from './recipe-pose';

/** A key's turn as a quaternion in the model's own axes. */
export function keyTurn([x, y, z]: Turn): THREE.Quaternion {
  const r = THREE.MathUtils.degToRad;
  return new THREE.Quaternion().setFromEuler(new THREE.Euler(r(x), r(y), r(z), 'XYZ'));
}

/** `a` wrapped to -180 to 180 degrees. */
const wrapped = (a: number) => a - 360 * Math.round(a / 360);

/**
 * A quaternion in the model's own axes as a key's turn, degrees. Every turn
 * has two such triples (x, y, z and x + 180, 180 - y, z + 180); with `near`,
 * the one closer to it, so a drag past 90 degrees of Y keeps the fields
 * moving on instead of flipping X and Z by 180.
 */
export function turnOfQuaternion(q: THREE.Quaternion, near?: Turn): Turn {
  const e = new THREE.Euler().setFromQuaternion(q, 'XYZ');
  const deg = THREE.MathUtils.radToDeg;
  const a: Turn = [deg(e.x), deg(e.y), deg(e.z)];
  const b: Turn = [wrapped(a[0] + 180), wrapped(180 - a[1]), wrapped(a[2] + 180)];
  const off = (t: Turn) =>
    near ? t.reduce((sum, v, i) => sum + wrapped(v - near[i]!) ** 2, 0) : 0;
  const d = (v: number) => Math.round(v * 10) / 10 + 0;
  const best = near && off(b) < off(a) ? b : a;
  return [d(best[0]), d(best[1]), d(best[2])];
}

const rotationOf = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());

function depth(o: THREE.Object3D): number {
  let n = 0;
  for (let p = o.parent; p; p = p.parent) n += 1;
  return n;
}

/** What the keys need from the preview's rig. */
export interface KeyRig {
  bone: (vrm: string) => THREE.Object3D | undefined;
  forward: THREE.Vector3;
  /** A bone's world rotation in the model's rest pose, or undefined for one that is not a bone. */
  rest: (bone: THREE.Object3D) => THREE.Quaternion | undefined;
}

/** How far `bone`'s parent is turned from its rest, in the world: none when it is not a bone. */
export function parentTurn(rig: KeyRig, bone: THREE.Object3D): THREE.Quaternion {
  const parent = bone.parent;
  const rest = parent ? rig.rest(parent) : undefined;
  if (!parent || !rest) return new THREE.Quaternion();
  return rotationOf(parent).multiply(rest.clone().invert());
}

function setWorldRotation(bone: THREE.Object3D, world: THREE.Quaternion) {
  const parent = bone.parent ? rotationOf(bone.parent) : new THREE.Quaternion();
  bone.quaternion.copy(parent.invert().multiply(world));
  bone.updateWorldMatrix(false, true);
}

/** Lays one pose key change's turns at this frame over the pose as it stands. */
export function layKeys(
  rig: KeyRig,
  turns: ReadonlyMap<string, Turn>,
  mode: LayerMode,
  weight: number
) {
  const w = Math.min(Math.max(weight, 0), 1);
  if (w <= 0) return;
  const basis = modelBasis(rig.forward);
  const toWorld = (q: THREE.Quaternion) =>
    basis.clone().multiply(q).multiply(basis.clone().invert());
  const bones = [...turns].flatMap(([vrm, turn]) => {
    const bone = rig.bone(vrm);
    return bone ? [{ bone, turn }] : [];
  });
  bones.sort((a, b) => depth(a.bone) - depth(b.bone));
  for (const { bone, turn } of bones) {
    const parent = parentTurn(rig, bone);
    const now = rotationOf(bone);
    if (mode === 'replace') {
      const rest = rig.rest(bone);
      if (!rest) continue;
      const want = parent
        .clone()
        .multiply(toWorld(keyTurn(turn)))
        .multiply(rest);
      setWorldRotation(bone, now.clone().slerp(want, w));
    } else {
      const added = toWorld(new THREE.Quaternion().slerp(keyTurn(turn), w));
      const step = parent.clone().multiply(added).multiply(parent.clone().invert());
      setWorldRotation(bone, step.multiply(now));
    }
  }
}

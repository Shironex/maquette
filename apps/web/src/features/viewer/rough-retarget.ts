/**
 * A rough retarget, live in the viewport: the same idea as the Blender bake
 * without its measures. The model's limbs turn to the way the clip's skeleton
 * stands at rest, then each frame every mapped bone takes its clip bone's
 * world rotation against that bone's rest, and the hips also move, scaled by
 * the hips height. The bake in Blender is the exact one.
 */
import * as THREE from 'three';

export interface RoughPair {
  bone: string;
  source: string | null;
  target: string | null;
}

/** The limbs that turn to line up, with the bone each points at. */
const LIMBS: Record<string, string> = {
  LeftArm: 'LeftForeArm',
  LeftForeArm: 'LeftHand',
  RightArm: 'RightForeArm',
  RightForeArm: 'RightHand',
  LeftUpLeg: 'LeftLeg',
  LeftLeg: 'LeftFoot',
  RightUpLeg: 'RightLeg',
  RightLeg: 'RightFoot',
};

const named = (root: THREE.Object3D, name: string | null) =>
  name ? root.getObjectByName(THREE.PropertyBinding.sanitizeNodeName(name)) : undefined;

/** Turns `bone` so its world rotation is `world`, under its parent as it stands. */
function setWorldRotation(bone: THREE.Object3D, world: THREE.Quaternion) {
  const parent = bone.parent?.getWorldQuaternion(new THREE.Quaternion()) ?? new THREE.Quaternion();
  bone.quaternion.copy(parent.invert().multiply(world));
  bone.updateWorldMatrix(false, true);
}

const positionOf = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());
const rotationOf = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());

interface Drive {
  source: THREE.Object3D;
  target: THREE.Object3D;
  sourceRestInverse: THREE.Quaternion;
  /** The target's world rotation in the lined up pose. */
  reference: THREE.Quaternion;
}

/**
 * Sets `model` up to follow `clip` played on `skeleton`, and returns what
 * poses it at a time, or null when no bone maps. Pairs go parents first,
 * as the humanoid map lists them.
 */
export function roughRetarget(
  skeleton: THREE.Object3D,
  model: THREE.Object3D,
  clip: THREE.AnimationClip,
  pairs: readonly RoughPair[]
): ((time: number) => void) | null {
  skeleton.updateWorldMatrix(true, true);
  model.updateWorldMatrix(true, true);
  const found = pairs.flatMap(p => {
    const source = named(skeleton, p.source);
    const target = named(model, p.target);
    return source && target ? [{ bone: p.bone, source, target }] : [];
  });
  if (!found.length) return null;
  const partner = (bone: string) => found.find(f => f.bone === bone);
  const hips = partner('Hips');
  const sourceHips = hips ? positionOf(hips.source) : null;
  const targetHips = hips ? positionOf(hips.target) : null;
  const scale = sourceHips && targetHips && sourceHips.y > 1e-4 ? targetHips.y / sourceHips.y : 1;

  const drives: Drive[] = found.map(f => {
    const next = LIMBS[f.bone] ? partner(LIMBS[f.bone]!) : undefined;
    if (next) {
      const want = positionOf(next.source).sub(positionOf(f.source)).normalize();
      const have = positionOf(next.target).sub(positionOf(f.target)).normalize();
      const turn = new THREE.Quaternion().setFromUnitVectors(have, want);
      setWorldRotation(f.target, turn.multiply(rotationOf(f.target)));
    }
    return {
      source: f.source,
      target: f.target,
      sourceRestInverse: rotationOf(f.source).invert(),
      reference: rotationOf(f.target),
    };
  });

  const mixer = new THREE.AnimationMixer(skeleton);
  mixer.clipAction(clip).play();
  const delta = new THREE.Quaternion();
  return time => {
    mixer.setTime(time);
    skeleton.updateWorldMatrix(true, true);
    for (const d of drives) {
      delta.copy(rotationOf(d.source)).multiply(d.sourceRestInverse);
      setWorldRotation(d.target, delta.clone().multiply(d.reference));
    }
    if (!hips || !sourceHips || !targetHips) return;
    const moved = positionOf(hips.source).sub(sourceHips).multiplyScalar(scale);
    const world = targetHips.clone().add(moved);
    hips.target.position.copy(hips.target.parent?.worldToLocal(world) ?? world);
    hips.target.updateWorldMatrix(false, true);
  };
}

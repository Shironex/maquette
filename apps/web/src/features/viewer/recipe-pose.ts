/**
 * A recipe's changes laid roughly over a clip already posed on a model:
 * reach is a two-bone IK toward its blank, look at turns the head or neck to
 * it, nudge adds a turn or a move on one bone and wave swings an arm on a
 * sine. Each eases in and out over its part of the clip. The bake solves
 * the same changes exactly in Blender; this is only a look before it.
 */
import * as THREE from 'three';
import type { LayerKind, RecipeLayer } from '@/ipc/client';

/** The share of a change's range it eases in over, and out over. */
export const RAMP = 0.25;
/** How far look at turns a bone, in radians, so a target behind does not twist the neck round. */
export const MAX_LOOK = THREE.MathUtils.degToRad(80);

const smooth = (x: number) => (x <= 0 ? 0 : x >= 1 ? 1 : x * x * (3 - 2 * x));

/**
 * How much of a change shows at `share` of the clip, 0 to 1. A reach eases
 * in over the first half of its range and holds to its end, where the hand
 * meets the target; the other kinds ease in and out over a quarter each.
 */
export function envelope(kind: LayerKind, start: number, end: number, share: number): number {
  if (!(end > start) || share < start || share > end) return 0;
  const x = (share - start) / (end - start);
  if (kind === 'reach') return smooth(x / 0.5);
  return Math.min(smooth(x / RAMP), smooth((1 - x) / RAMP));
}

export interface TwoBoneSolve {
  /** Where the middle joint (elbow, knee) goes. */
  mid: THREE.Vector3;
  /** Where the end (hand, foot) goes: on the target, or as near as the limb reaches. */
  end: THREE.Vector3;
  reached: boolean;
}

/** Any direction at right angles to `dir`. */
function anyPerpendicular(dir: THREE.Vector3): THREE.Vector3 {
  const axis = Math.abs(dir.y) < 0.9 ? new THREE.Vector3(0, 1, 0) : new THREE.Vector3(1, 0, 0);
  return new THREE.Vector3().crossVectors(dir, axis).normalize();
}

/**
 * Where a two-bone limb's middle and end go for its end to reach `target`,
 * keeping both bone lengths and the side the middle bends to now.
 */
export function solveTwoBone(
  root: THREE.Vector3,
  mid: THREE.Vector3,
  end: THREE.Vector3,
  target: THREE.Vector3
): TwoBoneSolve {
  const upper = mid.distanceTo(root);
  const lower = end.distanceTo(mid);
  const toTarget = target.clone().sub(root);
  const distance = toTarget.length();
  if (distance < 1e-6 || upper < 1e-6 || lower < 1e-6)
    return { mid: mid.clone(), end: end.clone(), reached: false };
  const dir = toTarget.divideScalar(distance);
  const reach = THREE.MathUtils.clamp(distance, Math.abs(upper - lower), upper + lower);
  const cos = THREE.MathUtils.clamp(
    (upper * upper + reach * reach - lower * lower) / (2 * upper * reach),
    -1,
    1
  );
  const sin = Math.sqrt(1 - cos * cos);
  const offset = mid.clone().sub(root);
  const bend = offset.sub(dir.clone().multiplyScalar(offset.dot(dir)));
  const side = bend.lengthSq() > 1e-10 ? bend.normalize() : anyPerpendicular(dir);
  return {
    mid: root
      .clone()
      .addScaledVector(dir, upper * cos)
      .addScaledVector(side, upper * sin),
    end: root.clone().addScaledVector(dir, reach),
    reached: distance <= upper + lower + 1e-6,
  };
}

const positionOf = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());
const rotationOf = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());

/** Turns `bone` so its world rotation is `world`, under its parent as it stands. */
function setWorldRotation(bone: THREE.Object3D, world: THREE.Quaternion) {
  const parent = bone.parent ? rotationOf(bone.parent) : new THREE.Quaternion();
  bone.quaternion.copy(parent.invert().multiply(world));
  bone.updateWorldMatrix(false, true);
}

/** Turns `bone` in world space by `turn`, `amount` of the way (0 to 1). */
function turnBy(bone: THREE.Object3D, turn: THREE.Quaternion, amount = 1) {
  const part = new THREE.Quaternion().slerp(turn, amount);
  setWorldRotation(bone, part.multiply(rotationOf(bone)));
}

/** Turns `bone` so a point it carries moves from `now` toward `want`. */
function swing(bone: THREE.Object3D, now: THREE.Vector3, want: THREE.Vector3) {
  const at = positionOf(bone);
  const from = now.clone().sub(at);
  const to = want.clone().sub(at);
  if (from.lengthSq() < 1e-12 || to.lengthSq() < 1e-12) return;
  turnBy(bone, new THREE.Quaternion().setFromUnitVectors(from.normalize(), to.normalize()));
}

/** The bones a recipe moves, found on the model by VRM name. */
export interface RecipeRig {
  bone: (vrm: string) => THREE.Object3D | undefined;
  /** A reach's upper and lower bones for its hand or foot. */
  limb: (end: string) => { upper: THREE.Object3D; lower: THREE.Object3D } | undefined;
  /** The way the model faces, in world space. */
  forward: THREE.Vector3;
  /** Each look bone's facing in its own space, read from the rest pose. */
  facing: (bone: THREE.Object3D) => THREE.Vector3;
}

const num = (v: number | null | undefined, or = 0) =>
  typeof v === 'number' && Number.isFinite(v) ? v : or;

/** Moves a reach's limb `amount` of the way to `target`. */
export function reachToward(
  limb: { upper: THREE.Object3D; lower: THREE.Object3D },
  end: THREE.Object3D,
  target: THREE.Vector3,
  amount: number
) {
  const root = positionOf(limb.upper);
  const mid = positionOf(limb.lower);
  const tip = positionOf(end);
  const solve = solveTwoBone(root, mid, tip, tip.clone().lerp(target, amount));
  swing(limb.upper, mid, solve.mid);
  swing(limb.lower, positionOf(end), solve.end);
}

function lookToward(rig: RecipeRig, bone: THREE.Object3D, target: THREE.Vector3, amount: number) {
  const facing = rig.facing(bone).clone().applyQuaternion(rotationOf(bone));
  const want = target.clone().sub(positionOf(bone));
  if (want.lengthSq() < 1e-12) return;
  const turn = new THREE.Quaternion().setFromUnitVectors(facing.normalize(), want.normalize());
  const angle = 2 * Math.acos(THREE.MathUtils.clamp(Math.abs(turn.w), 0, 1));
  const limit = angle > MAX_LOOK ? MAX_LOOK / angle : 1;
  turnBy(bone, turn, amount * limit);
}

function nudge(layer: RecipeLayer, bone: THREE.Object3D, amount: number) {
  const { rotation, offset } = layer.settings;
  if (rotation) {
    const [x, y, z] = rotation.map(v => THREE.MathUtils.degToRad(num(v)));
    const turn = new THREE.Quaternion().setFromEuler(new THREE.Euler(x, y, z));
    bone.quaternion.multiply(new THREE.Quaternion().slerp(turn, amount));
  }
  if (offset && bone.parent) {
    const world = positionOf(bone).add(
      new THREE.Vector3(num(offset[0]), num(offset[1]), num(offset[2])).multiplyScalar(amount)
    );
    bone.position.copy(bone.parent.worldToLocal(world));
  }
  bone.updateWorldMatrix(false, true);
}

function wave(
  rig: RecipeRig,
  layer: RecipeLayer,
  bone: THREE.Object3D,
  share: number,
  amount: number
) {
  const s = layer.settings;
  const start = num(s.start);
  const span = num(s.end, 1) - start;
  const x = span > 0 ? (share - start) / span : 0;
  const angle =
    THREE.MathUtils.degToRad(num(s.amount)) * Math.sin(2 * Math.PI * num(s.cycles) * x) * amount;
  turnBy(bone, new THREE.Quaternion().setFromAxisAngle(rig.forward, angle));
}

/**
 * Lays `layers` over the pose the model has now, in order, at `share` of
 * the clip. A change whose bone the model lacks or whose blank is open
 * leaves the pose alone.
 */
export function poseRecipe(
  rig: RecipeRig,
  layers: readonly RecipeLayer[],
  targets: ReadonlyMap<string, THREE.Vector3>,
  share: number
) {
  for (const layer of layers) {
    const s = layer.settings;
    const amount = num(s.weight, 1) * envelope(layer.kind, num(s.start), num(s.end, 1), share);
    const bone = rig.bone(layer.bone);
    if (amount <= 0 || !bone) continue;
    const target = s.target ? targets.get(s.target) : undefined;
    if (layer.kind === 'reach') {
      const limb = rig.limb(layer.bone);
      if (limb && target) reachToward(limb, bone, target, amount);
    } else if (layer.kind === 'look-at') {
      if (target) lookToward(rig, bone, target, amount);
    } else if (layer.kind === 'nudge') nudge(layer, bone, amount);
    else wave(rig, layer, bone, share, amount);
  }
}

/** A limb's bend: which way the middle joint points off the line from root to end. */
export function bendOf(root: THREE.Vector3, mid: THREE.Vector3, end: THREE.Vector3) {
  return new THREE.Vector3().subVectors(mid, root).cross(new THREE.Vector3().subVectors(end, root));
}

export { positionOf as worldPosition };

/** A blank as the preview places it: a point where it says, a prop in a slot in front. */
export interface RecipeMarker {
  id: string;
  kind: 'prop' | 'point';
  /** A point blank's point, metres from the model's feet with Y up; null for a prop. */
  point: readonly [number, number, number] | null;
}

/**
 * Where each blank's target sits. A point is where it says, from the spot
 * the model stands on. A prop has no place in a recipe, so the preview puts
 * each in a slot in front of the model, within reach of a hand, alternating sides.
 */
export function targetsOf(markers: readonly RecipeMarker[], height: number) {
  const out = new Map<string, THREE.Vector3>();
  let slot = 0;
  for (const m of markers) {
    if (m.point) {
      out.set(m.id, new THREE.Vector3(m.point[0], m.point[1], m.point[2]));
      continue;
    }
    if (m.kind !== 'prop') continue;
    const side = slot % 2 === 0 ? -1 : 1;
    const row = Math.floor(slot / 2);
    out.set(
      m.id,
      new THREE.Vector3(side * 0.22 * height, 0.65 * height, (0.18 + 0.1 * row) * height)
    );
    slot += 1;
  }
  return out;
}

/**
 * A recipe's pins held roughly on the preview, after every other change, as
 * the bake holds them exactly in Blender: at its first frame a pin takes its
 * spot (where the hand or foot is then, or its blank's target), and from
 * there two-bone IK keeps the bone on it, eased in and out over the frames
 * the core gives (state/recipe-pins.ts). A hand keeps the turn it had at its
 * first frame; a foot keeps it whole (toe and heel), only its heading
 * (heel, the toe free to roll) or none (off). Plus the live check behind
 * "She can stay on it": how far the hips move while it holds, and whether
 * the leg or arm still reaches the spot.
 */
import * as THREE from 'three';
import type { RecipeLayer } from '@/ipc/client';
import type { GripSpan } from '@/state/recipe-grips';
import { pinAmount, type PinCheck, type PinSpan } from '@/state/recipe-pins';
import type { Playable } from '@/state/recipe-timeline';
import {
  poseRecipe,
  reachToward,
  setWorldRotation,
  worldPosition,
  type RecipeRig,
} from './recipe-pose';

/** Where a pin stays: a point in the world and the bone's turn there. */
export interface PinSpot {
  position: THREE.Vector3;
  rotation: THREE.Quaternion;
}

/** Poses the bake's frame `frame` with no change laid over, and answers where the changes stand there. */
export type PoseAt = (frame: number) => number;

const UP = new THREE.Vector3(0, 1, 0);
const AXIS = new THREE.Vector3(0, 1, 0);
const rotationOf = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());
const level = (v: THREE.Vector3) => v.setY(0);

/** `now` turned about the upright so the bone points the way `want` had it, level. */
function headingKept(now: THREE.Quaternion, want: THREE.Quaternion): THREE.Quaternion {
  const to = level(AXIS.clone().applyQuaternion(want));
  const from = level(AXIS.clone().applyQuaternion(now));
  if (to.lengthSq() < 1e-10 || from.lengthSq() < 1e-10) return now.clone();
  const angle = Math.atan2(from.clone().cross(to).dot(UP), from.dot(to));
  return new THREE.Quaternion().setFromAxisAngle(UP, angle).multiply(now);
}

/**
 * Holds each pin of `spans` that shows at bake frame `frame` on its spot,
 * in order, over the pose the model has now.
 */
export function holdPins(
  rig: RecipeRig,
  spans: readonly PinSpan[],
  spots: ReadonlyMap<string, PinSpot>,
  frame: number
) {
  for (const pin of spans) {
    const spot = spots.get(pin.layer);
    const amount = Math.min(pinAmount(pin, frame), 1);
    const bone = rig.bone(pin.bone);
    const limb = rig.limb(pin.bone);
    if (!spot || amount <= 0 || !bone || !limb) continue;
    reachToward(limb, bone, spot.position, amount);
    if (pin.flat === 'off') continue;
    const now = rotationOf(bone);
    const want = pin.flat === 'heel' ? headingKept(now, spot.rotation) : spot.rotation;
    setWorldRotation(bone, now.slerp(want, amount));
  }
}

/**
 * Where each pin stays: at its first frame, with every change (pose keys
 * on `keysOn`, the grips of `grips`) and the pins before it laid over, the bone where it is then (or its blank's target)
 * and its turn. The caller poses the frame on show again afterwards.
 */
export function pinSpots(
  poseAt: PoseAt,
  rig: RecipeRig,
  layers: readonly RecipeLayer[],
  targets: ReadonlyMap<string, THREE.Vector3>,
  spans: readonly PinSpan[],
  keysOn: Playable | null = null,
  grips: readonly GripSpan[] = []
): Map<string, PinSpot> {
  const spots = new Map<string, PinSpot>();
  spans.forEach((pin, i) => {
    const bone = rig.bone(pin.bone);
    if (!bone) return;
    const share = poseAt(pin.first);
    poseRecipe(rig, layers, targets, share, keysOn, { spans: grips, frame: pin.first });
    holdPins(rig, spans.slice(0, i), spots, pin.first);
    const target = pin.target ? targets.get(pin.target) : undefined;
    if (pin.target && !target) return;
    spots.set(pin.layer, {
      position: (target ?? worldPosition(bone)).clone(),
      rotation: rotationOf(bone),
    });
  });
  return spots;
}

/** The most frames a check poses per pin. */
const CHECK_SAMPLES = 48;

/**
 * Measures each pin over its frames: how far the hips travel and whether the
 * limb's root stays within its length of the spot. The caller poses the
 * frame on show again afterwards.
 */
export function checkPins(
  poseAt: PoseAt,
  rig: RecipeRig,
  layers: readonly RecipeLayer[],
  targets: ReadonlyMap<string, THREE.Vector3>,
  spans: readonly PinSpan[],
  spots: ReadonlyMap<string, PinSpot>,
  keysOn: Playable | null = null,
  grips: readonly GripSpan[] = []
): PinCheck[] {
  const out: PinCheck[] = [];
  const hips = rig.bone('hips');
  for (const pin of spans) {
    const spot = spots.get(pin.layer);
    const end = rig.bone(pin.bone);
    const limb = rig.limb(pin.bone);
    if (!spot || !end || !limb) continue;
    const length =
      worldPosition(limb.upper).distanceTo(worldPosition(limb.lower)) +
      worldPosition(limb.lower).distanceTo(worldPosition(end));
    const span = pin.last - pin.first;
    const step = Math.max(1, Math.ceil(span / CHECK_SAMPLES));
    let start: THREE.Vector3 | null = null;
    let travel = 0;
    let outFrame: number | null = null;
    for (let f = pin.first; f <= pin.last; f += step) {
      poseRecipe(rig, layers, targets, poseAt(f), keysOn, { spans: grips, frame: f });
      if (hips) {
        const at = worldPosition(hips);
        start ??= at.clone();
        travel = Math.max(travel, at.distanceTo(start));
      }
      const reach = worldPosition(limb.upper).distanceTo(spot.position);
      if (outFrame == null && pinAmount(pin, f) > 0 && reach > length + 1e-3) outFrame = f;
    }
    out.push({ layer: pin.layer, hips: travel, limb: length, outFrame });
  }
  return out;
}

/** The pins a stage holds: each on the bake's frames, the bake's last frame, and who hears the checks. */
export interface StagePins {
  spans: readonly PinSpan[];
  frames: number;
  onChecks?: (checks: PinCheck[]) => void;
}

/**
 * How a set up stage poses bake frame `frame`: a joined recipe plays its
 * timeline there; one clip played whole poses that share of the clip (the
 * bake's frames are the clip's).
 */
export function poseAtOf(
  setup: { frameAt: (share: number) => void; playAt?: (playhead: number) => number },
  frames: number
): PoseAt {
  return frame => {
    const share = frames > 0 ? frame / frames : 0;
    if (setup.playAt) return setup.playAt(share);
    setup.frameAt(share);
    return share;
  };
}

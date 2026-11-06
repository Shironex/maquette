import { useEffect, useEffectEvent, useMemo, useState } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import type { RecipeLayer } from '@/ipc/client';
import { LOOK_BONES, limbOf, mixamoName } from '@/state/recipe-bones';
import type { ReachCheck } from '@/state/recipe-review';
import { checkReaches } from '../check-reaches';
import {
  poseRecipe,
  rigForward,
  targetsOf,
  type RecipeMarker,
  type RecipeRig,
} from '../recipe-pose';
import { roughRetarget, type RoughPair } from '../rough-retarget';
import type { LoadedMotion } from '../MotionStage/Scene/Scene.types';

/** The model lined up to follow the clip (or in its rest pose), and what poses it at a share of the clip. */
export interface RecipeSetup {
  rig: RecipeRig;
  /** Puts the clip's pose at `share` back on the model, with no change laid over. */
  frameAt: (share: number) => void;
  /** Stops the clip's mixer and puts the model and skeleton back as they came. */
  stop: () => void;
}

const rotationOf = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());

type Snapshot = (readonly [THREE.Object3D, THREE.Vector3, THREE.Quaternion])[];

/** Each loaded scene's bones as they came, taken the first time it is set up, before any pose. */
const asLoaded = new WeakMap<THREE.Object3D, Snapshot>();

/** The bones under `root` as they stand; the scene itself is left to the framing. */
function take(root: THREE.Object3D): Snapshot {
  const out: Snapshot = [];
  root.traverse(o => {
    if (o instanceof THREE.Bone) out.push([o, o.position.clone(), o.quaternion.clone()] as const);
  });
  return out;
}

function put(snapshot: Snapshot) {
  for (const [o, position, quaternion] of snapshot) {
    o.position.copy(position);
    o.quaternion.copy(quaternion);
  }
}

/** Puts `root` back as it came, remembering that the first time it is seen. */
function unposed(root: THREE.Object3D) {
  const known = asLoaded.get(root);
  if (known) put(known);
  else asLoaded.set(root, take(root));
  root.updateWorldMatrix(true, true);
}

/** No clip to follow: the model holds its rest pose, as the bake does. */
const REST_POSE = { frameAt: () => undefined, stop: () => undefined };

/** The base clip of `skeleton` played on `model` through the bone pairs, or null. */
function following(model: LoadedMotion, skeleton: LoadedMotion, pairs: readonly RoughPair[]) {
  const clip = skeleton.clips[0];
  if (!clip) return null;
  unposed(skeleton.scene);
  const pose = roughRetarget(skeleton.scene, model.scene, clip, pairs);
  if (!pose) return null;
  return {
    frameAt: (share: number) => pose(share * clip.duration),
    stop: () => {
      pose.stop();
      unposed(skeleton.scene);
    },
  };
}

function setUp(
  model: LoadedMotion,
  skeleton: LoadedMotion | 'rest',
  pairs: readonly RoughPair[]
): RecipeSetup | null {
  // A model or clip set up before (another clip, the same model) starts again as it came.
  unposed(model.scene);
  const bone = (vrm: string) => {
    const name = mixamoName(vrm);
    const target = pairs.find(p => p.bone === name)?.target;
    return target
      ? model.scene.getObjectByName(THREE.PropertyBinding.sanitizeNodeName(target))
      : undefined;
  };
  // Read from the rig as it came, before the retarget lines its limbs up.
  const forward = rigForward(bone);
  const base = skeleton === 'rest' ? REST_POSE : following(model, skeleton, pairs);
  if (!base) return null;
  // Every frame starts from the lined up rest, so a nudge's move never adds up.
  const rest = take(model.scene);
  const facings = new Map<THREE.Object3D, THREE.Vector3>();
  for (const vrm of LOOK_BONES) {
    const b = bone(vrm);
    if (b) facings.set(b, forward.clone().applyQuaternion(rotationOf(b).invert()));
  }
  const rig: RecipeRig = {
    bone,
    limb: end => {
      const names = limbOf(end);
      const upper = names ? bone(names.upper) : undefined;
      const lower = names ? bone(names.lower) : undefined;
      return upper && lower ? { upper, lower } : undefined;
    },
    forward,
    facing: b => facings.get(b) ?? new THREE.Vector3(0, 0, 1),
  };
  return {
    rig,
    frameAt: share => {
      put(rest);
      model.scene.updateWorldMatrix(true, true);
      base.frameAt(share);
    },
    stop: () => {
      base.stop();
      unposed(model.scene);
    },
  };
}

/**
 * Lines `model` up with the clip of `skeleton` through the bone pairs, or
 * holds its rest pose for `'rest'`, afresh for each new one.
 */
export function useRecipeSetup(
  model: LoadedMotion | null,
  skeleton: LoadedMotion | 'rest' | null,
  pairs: readonly RoughPair[]
): RecipeSetup | null {
  const [setup, setSetup] = useState<RecipeSetup | null>(null);
  useEffect(() => {
    const next = model && skeleton ? setUp(model, skeleton, pairs) : null;
    setSetup(next);
    return () => next?.stop();
  }, [model, skeleton, pairs]);
  return setup;
}

/**
 * Where each blank's target sits, from the model's height once it is framed
 * and the way its rig faces (+Z until it is set up).
 */
export function useTargets(
  markers: readonly RecipeMarker[],
  box: THREE.Box3 | null,
  forward: THREE.Vector3 | null
) {
  const key = JSON.stringify(markers);
  const height = box ? box.max.y - box.min.y : 0;
  const [x, z] = forward ? [forward.x, forward.z] : [0, 1];
  return useMemo(
    () => targetsOf(JSON.parse(key) as RecipeMarker[], height || 1.7, new THREE.Vector3(x, 0, z)),
    [key, height, x, z]
  );
}

/**
 * Poses the clip at `share` each frame with the changes laid over it,
 * moving the playhead while it plays, and measures each reach's end again
 * whenever the changes or targets change.
 */
export function useRecipeFrame(
  setup: RecipeSetup | null,
  layers: readonly RecipeLayer[],
  targets: ReadonlyMap<string, THREE.Vector3>,
  play: { share: number; playing: boolean; onTick: (delta: number) => void },
  onChecks?: (checks: ReachCheck[]) => void
) {
  const report = useEffectEvent((checks: ReachCheck[]) => onChecks?.(checks));
  useEffect(() => {
    if (setup) report(checkReaches(setup.frameAt, setup.rig, layers, targets));
  }, [setup, layers, targets]);
  useFrame((_, delta) => {
    if (!setup) return;
    if (play.playing) play.onTick(delta);
    setup.frameAt(play.share);
    poseRecipe(setup.rig, layers, targets, play.share);
  });
}

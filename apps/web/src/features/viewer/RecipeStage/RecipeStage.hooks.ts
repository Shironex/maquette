import { useEffect, useEffectEvent, useMemo, useState } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import type { RecipeLayer } from '@/ipc/client';
import { LOOK_BONES, limbOf, mixamoName } from '@/state/recipe-bones';
import type { ReachCheck } from '@/state/recipe-review';
import { checkReaches } from '../check-reaches';
import { poseRecipe, targetsOf, type RecipeMarker, type RecipeRig } from '../recipe-pose';
import { roughRetarget, type RoughPair } from '../rough-retarget';
import type { LoadedMotion } from '../MotionStage/Scene/Scene.types';

/** The model lined up to follow the clip, and what poses it at a share of the clip. */
export interface RecipeSetup {
  rig: RecipeRig;
  /** Puts the clip's pose at `share` back on the model, with no change laid over. */
  frameAt: (share: number) => void;
}

const rotationOf = (o: THREE.Object3D) => o.getWorldQuaternion(new THREE.Quaternion());

function setUp(
  model: LoadedMotion,
  skeleton: LoadedMotion,
  pairs: readonly RoughPair[]
): RecipeSetup | null {
  const clip = skeleton.clips[0];
  if (!clip) return null;
  const pose = roughRetarget(skeleton.scene, model.scene, clip, pairs);
  if (!pose) return null;
  // Every frame starts from the lined up rest, so a nudge's move never adds up.
  const rest = model.bones.map(b => [b, b.position.clone(), b.quaternion.clone()] as const);
  const bone = (vrm: string) => {
    const name = mixamoName(vrm);
    const target = pairs.find(p => p.bone === name)?.target;
    return target
      ? model.scene.getObjectByName(THREE.PropertyBinding.sanitizeNodeName(target))
      : undefined;
  };
  const forward = new THREE.Vector3(0, 0, 1).applyQuaternion(rotationOf(model.scene)).normalize();
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
      for (const [b, position, quaternion] of rest) {
        b.position.copy(position);
        b.quaternion.copy(quaternion);
      }
      model.scene.updateWorldMatrix(true, true);
      pose(share * clip.duration);
    },
  };
}

/** Lines `model` up with the clip of `skeleton` through the bone pairs, afresh for each new one. */
export function useRecipeSetup(
  model: LoadedMotion | null,
  skeleton: LoadedMotion | null,
  pairs: readonly RoughPair[]
): RecipeSetup | null {
  const [setup, setSetup] = useState<RecipeSetup | null>(null);
  useEffect(() => {
    setSetup(model && skeleton ? setUp(model, skeleton, pairs) : null);
  }, [model, skeleton, pairs]);
  return setup;
}

/** Where each blank's target sits, from the model's height once it is framed. */
export function useTargets(markers: readonly RecipeMarker[], box: THREE.Box3 | null) {
  const key = JSON.stringify(markers);
  const height = box ? box.max.y - box.min.y : 0;
  return useMemo(() => targetsOf(JSON.parse(key) as RecipeMarker[], height || 1.7), [key, height]);
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

import { useEffect, useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { roughRetarget, type RoughPair } from '../rough-retarget';
import type { LoadedMotion } from '../MotionStage/Scene/Scene.types';

/**
 * Keeps `model` playing the first clip of `skeleton`, looping, through the
 * bone pairs. Each new model or clip lines the rig up afresh; it does not
 * carry the last clip's pose, since a changed clip file loads a new scene.
 */
export function useRoughPose(
  model: LoadedMotion | null,
  skeleton: LoadedMotion | null,
  pairs: readonly RoughPair[],
  onFrame?: (seconds: number) => void
) {
  const pose = useRef<((time: number) => void) | null>(null);
  const time = useRef(0);
  const length = useRef(0);
  useEffect(() => {
    pose.current = null;
    time.current = 0;
    const clip = skeleton?.clips[0];
    if (!model || !skeleton || !clip) return;
    length.current = clip.duration;
    pose.current = roughRetarget(skeleton.scene, model.scene, clip, pairs);
  }, [model, skeleton, pairs]);
  useFrame((_, delta) => {
    if (!pose.current || length.current <= 0) return;
    time.current = (time.current + delta) % length.current;
    pose.current(time.current);
    onFrame?.(time.current);
  });
}

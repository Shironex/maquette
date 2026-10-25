import { useFrame, useThree } from '@react-three/fiber';
import { useMemo } from 'react';
import * as THREE from 'three';
import type { CameraStop } from './manifest';
import { createPath, createShot, shotOf } from './path';

const DESIGN_ASPECT = 16 / 10;

function fitFov(fov: number, aspect: number): number {
  if (aspect >= DESIGN_ASPECT) return fov;
  const half = THREE.MathUtils.degToRad(fov / 2);
  return THREE.MathUtils.radToDeg(2 * Math.atan((Math.tan(half) * DESIGN_ASPECT) / aspect));
}

export interface CameraRigOptions {
  /** Tour progress in stops, already eased (see useProgress). */
  progress: React.RefObject<number>;
  stops: readonly CameraStop[];
  reducedMotion: boolean;
  /** Holds this framing instead of following the tour (poster capture). */
  fixed?: CameraStop;
}

/** Puts the camera on the tour's path at the current progress; reduced motion cuts between stops. */
export function useCameraRig({ progress, stops, reducedMotion, fixed }: CameraRigOptions) {
  const camera = useThree(state => state.camera) as THREE.PerspectiveCamera;
  const path = useMemo(() => createPath(stops), [stops]);
  const shot = useMemo(createShot, []);

  useFrame(state => {
    if (fixed) shotOf(fixed, shot);
    else path.shotAt(progress.current, shot, reducedMotion);
    camera.position.copy(shot.position);
    camera.fov = fitFov(shot.fov, state.size.width / Math.max(1, state.size.height));
    camera.lookAt(shot.target);
    camera.updateProjectionMatrix();
  });
}

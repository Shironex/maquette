import { useFrame, useThree } from '@react-three/fiber';
import { useRef } from 'react';
import * as THREE from 'three';
import type { CameraStop, StopId } from './manifest';

const DESIGN_ASPECT = 16 / 10;
/** Damping rate; higher settles faster. */
const LAMBDA = 3.4;
const SETTLED = 1e-4;

function fitFov(fov: number, aspect: number): number {
  if (aspect >= DESIGN_ASPECT) return fov;
  const half = THREE.MathUtils.degToRad(fov / 2);
  return THREE.MathUtils.radToDeg(2 * Math.atan((Math.tan(half) * DESIGN_ASPECT) / aspect));
}

export interface CameraRigOptions {
  stop: React.RefObject<StopId>;
  stops: readonly CameraStop[];
  reducedMotion: boolean;
  /** Holds this framing instead of following the stops (poster capture). */
  fixed?: CameraStop;
}

/** Eases the camera toward the current stop each frame, redrawing until it settles. */
export function useCameraRig({ stop, stops, reducedMotion, fixed }: CameraRigOptions) {
  const camera = useThree(state => state.camera) as THREE.PerspectiveCamera;
  const invalidate = useThree(state => state.invalidate);
  const look = useRef<THREE.Vector3 | null>(null);
  const goal = useRef({ position: new THREE.Vector3(), target: new THREE.Vector3() });

  useFrame((state, delta) => {
    const shot = fixed ?? stops.find(s => s.id === stop.current) ?? stops[0];
    if (!shot) return;
    const { position, target } = goal.current;
    position.set(...shot.position);
    target.set(...shot.target);
    const fov = fitFov(shot.fov, state.size.width / Math.max(1, state.size.height));

    if (!look.current || reducedMotion) {
      look.current = target.clone();
      camera.position.copy(position);
      camera.fov = fov;
    } else {
      // Frame rate independent exponential easing; capped so a stalled tab does not jump.
      const t = 1 - Math.exp(-LAMBDA * Math.min(delta, 0.1));
      camera.position.lerp(position, t);
      look.current.lerp(target, t);
      camera.fov += (fov - camera.fov) * t;
    }
    camera.lookAt(look.current);
    camera.updateProjectionMatrix();

    const left =
      camera.position.distanceToSquared(position) +
      look.current.distanceToSquared(target) +
      (camera.fov - fov) ** 2 * 1e-4;
    if (left > SETTLED) invalidate();
  });
}

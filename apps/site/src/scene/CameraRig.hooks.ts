import { useFrame, useThree } from '@react-three/fiber';
import { useEffect, useMemo } from 'react';
import * as THREE from 'three';
import type { CameraStop } from './manifest';
import { createPath, createShot, shotOf } from './path';

const DESIGN_ASPECT = 16 / 10;

/** Widens the stops' vertical field of view on a window narrower than 16:10, so the subject stays in frame. */
function fitFov(fov: number, aspect: number): number {
  if (aspect >= DESIGN_ASPECT) return fov;
  const half = THREE.MathUtils.degToRad(fov / 2);
  return THREE.MathUtils.radToDeg(2 * Math.atan((Math.tan(half) * DESIGN_ASPECT) / aspect));
}

/**
 * The camera breathing at rest: a slow sway of a few millimetres, made of
 * two sines per axis so it never visibly repeats. Metres, and per second.
 */
const BREATH = { amount: 0.006, look: 0.004, speed: 0.31 };
/** How far the pointer swings the camera around its target, in radians. */
const PARALLAX = { yaw: 0.035, pitch: 0.02 };
/** How fast the pointer follower catches up, per second. */
const POINTER_EASE = 2.5;

export interface CameraRigOptions {
  /** Tour progress in stops, already eased (see useProgress). */
  progress: React.RefObject<number>;
  stops: readonly CameraStop[];
  /** Poster capture: exact stop framings, no breathing and no pointer. */
  still: boolean;
  /** Holds this framing instead of following the tour (poster capture). */
  fixed?: CameraStop;
}

/** The pointer across the window, -1 to 1, from a mouse only (never touch or pen). */
function usePointer(enabled: boolean) {
  const pointer = useMemo(() => ({ x: 0, y: 0, sx: 0, sy: 0 }), []);
  useEffect(() => {
    if (!enabled) return;
    const onMove = (event: PointerEvent) => {
      if (event.pointerType !== 'mouse') return;
      pointer.x = (event.clientX / window.innerWidth) * 2 - 1;
      pointer.y = (event.clientY / window.innerHeight) * 2 - 1;
    };
    const onLeave = () => {
      pointer.x = 0;
      pointer.y = 0;
    };
    window.addEventListener('pointermove', onMove, { passive: true });
    document.documentElement.addEventListener('pointerleave', onLeave);
    return () => {
      window.removeEventListener('pointermove', onMove);
      document.documentElement.removeEventListener('pointerleave', onLeave);
    };
  }, [enabled, pointer]);
  return pointer;
}

/**
 * Flies the camera along the tour's path at the current progress, breathing
 * a little at rest and leaning after the pointer; poster capture holds the
 * exact stop framings.
 */
export function useCameraRig({ progress, stops, still, fixed }: CameraRigOptions) {
  const camera = useThree(state => state.camera) as THREE.PerspectiveCamera;
  const path = useMemo(() => createPath(stops), [stops]);
  const shot = useMemo(createShot, []);
  const side = useMemo(() => new THREE.Vector3(), []);
  const lift = useMemo(() => new THREE.Vector3(), []);
  const pointer = usePointer(!still);

  useFrame((state, delta) => {
    if (fixed) shotOf(fixed, shot);
    else path.shotAt(progress.current, shot, still);

    if (!still) {
      const t = state.clock.elapsedTime * BREATH.speed;
      shot.position.x += (Math.sin(t) + 0.5 * Math.sin(t * 2.3 + 1)) * BREATH.amount;
      shot.position.y += (Math.sin(t * 1.3 + 2) + 0.4 * Math.sin(t * 3.1)) * BREATH.amount * 0.6;
      shot.position.z += Math.sin(t * 0.7 + 4) * BREATH.amount;
      shot.target.x += Math.sin(t * 0.9 + 3) * BREATH.look;
      shot.target.y += Math.sin(t * 1.1 + 5) * BREATH.look * 0.6;

      const ease = 1 - Math.exp(-POINTER_EASE * Math.min(delta, 0.05));
      pointer.sx += (pointer.x - pointer.sx) * ease;
      pointer.sy += (pointer.y - pointer.sy) * ease;
      const distance = shot.position.distanceTo(shot.target);
      side.subVectors(shot.target, shot.position).cross(camera.up).normalize();
      lift.subVectors(shot.target, shot.position).cross(side).normalize().negate();
      shot.position.addScaledVector(side, pointer.sx * PARALLAX.yaw * distance);
      shot.position.addScaledVector(lift, -pointer.sy * PARALLAX.pitch * distance);
    }

    camera.position.copy(shot.position);
    camera.fov = fitFov(shot.fov, state.size.width / Math.max(1, state.size.height));
    camera.lookAt(shot.target);
    camera.updateProjectionMatrix();
  });
}

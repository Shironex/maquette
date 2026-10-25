import * as THREE from 'three';
import { deg } from './placement';

/** The posters' frame. */
export const ASPECT = 16 / 9;

/**
 * The vertical field of view that shows a 16:9 poster's whole frame: the
 * stop's own when the view is at least as wide, else wider, so the frame
 * fits the width (the letterbox marks it).
 */
export function fitFov(fov: number, aspect: number): number {
  if (!(aspect > 0) || aspect >= ASPECT) return fov;
  const half = Math.atan((Math.tan(deg(fov) / 2) * ASPECT) / aspect);
  return THREE.MathUtils.radToDeg(half * 2);
}

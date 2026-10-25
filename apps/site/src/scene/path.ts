/**
 * The camera's path through the tour. Each stop plays its move (see
 * ./cameras.ts) across a window of the scroll around it: keyframes at the
 * move's start, the stop's own framing and the move's end. One spline runs
 * through every keyframe of every stop, so the camera never stops between
 * a move and the travel to the next stop; a stop without a move holds.
 */
import * as THREE from 'three';
import type { CameraMove, CameraStop } from './cameras';
import { nearestStop } from './progress';
import { createSpline } from './spline';

/** Half the scroll window, in stops, a stop's move plays across. */
export const MOVE = 0.25;

export interface Shot {
  position: THREE.Vector3;
  target: THREE.Vector3;
  /** Vertical field of view in degrees, for the stops' 16:10 framing. */
  fov: number;
}

export function createShot(): Shot {
  return { position: new THREE.Vector3(), target: new THREE.Vector3(), fov: 36 };
}

/** Copies a stop's framing into `out`. */
export function shotOf(stop: CameraStop, out: Shot): Shot {
  out.position.set(...stop.position);
  out.target.set(...stop.target);
  out.fov = stop.fov;
  return out;
}

const UP = new THREE.Vector3(0, 1, 0);

/** A stop's framing `t` of the way through its move (0 is the stop itself), as [position, target, fov]. */
export function moved(stop: CameraStop, t: number): number[] {
  const move: CameraMove = stop.move ?? {};
  const position = new THREE.Vector3(...stop.position);
  const target = new THREE.Vector3(...stop.target);
  if (move.orbit) {
    const pivot = new THREE.Vector3(...(move.pivot ?? stop.target));
    const angle = THREE.MathUtils.degToRad(move.orbit * t);
    for (const v of [position, target]) v.sub(pivot).applyAxisAngle(UP, angle).add(pivot);
  }
  if (move.dolly) {
    const toward = target.clone().sub(position).normalize();
    position.addScaledVector(toward, move.dolly * t);
  }
  if (move.rise) position.y += move.rise * t;
  if (move.truck) {
    const slide = new THREE.Vector3(...move.truck).multiplyScalar(t);
    position.add(slide);
    target.add(slide);
  }
  return [...position.toArray(), ...target.toArray(), stop.fov];
}

/**
 * Where each stop's keyframes sit (offset from the stop, in stops) and how
 * far through its move each is: the first stop plays forward from its
 * framing, the last one ends on it (over half the window, so the travel
 * into it is not rushed), the rest are centred on it.
 */
function windowOf(index: number, last: number): (readonly [number, number])[] {
  if (index === 0)
    return [
      [0, 0],
      [MOVE, 0.5],
    ];
  if (index === last)
    return [
      [-MOVE, -1],
      [-MOVE / 2, -0.5],
      [0, 0],
    ];
  return [
    [-MOVE, -0.5],
    [0, 0],
    [MOVE, 0.5],
  ];
}

export interface CameraPath {
  /**
   * The framing at tour progress `progress` (in stops), written into `out`.
   * With `cut` (reduced motion) it is the nearest stop's framing.
   */
  shotAt(progress: number, out: Shot, cut?: boolean): Shot;
}

export function createPath(stops: readonly CameraStop[]): CameraPath {
  const last = stops.length - 1;
  const keys: number[] = [];
  const values: number[][] = [];
  stops.forEach((stop, index) => {
    for (const [offset, t] of windowOf(index, last)) {
      keys.push(index + offset);
      values.push(moved(stop, t));
    }
  });
  const spline = createSpline(keys, values);
  const sample: number[] = [];

  return {
    shotAt(progress, out, cut = false) {
      const first = stops[0];
      if (!first) return out;
      if (cut) return shotOf(stops[nearestStop(progress, last)] ?? first, out);
      const [px = 0, py = 0, pz = 0, tx = 0, ty = 0, tz = 0, fov = first.fov] = spline.at(
        progress,
        sample
      );
      out.position.set(px, py, pz);
      out.target.set(tx, ty, tz);
      out.fov = fov;
      return out;
    },
  };
}

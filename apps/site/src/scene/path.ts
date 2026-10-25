/**
 * The camera's flight through the tour. Every stop adds keyframes: the poses
 * it flies through on its way in from the previous stop (`via`), then its
 * move (see ./cameras.ts) played slowly across a short window around its
 * own framing. One curve runs through every camera position and one through
 * every look target, and a monotone timing walks both: slow through each
 * stop's move, fast on the flights between, never stopping, so the camera
 * keeps travelling as the page scrolls.
 */
import * as THREE from 'three';
import type { CameraMove, CameraStop } from './cameras';
import { nearestStop } from './progress';
import { createMonotone, type Spline } from './spline';
import { createTrack, type Track } from './track';

/** Half the window, in stops, a stop's move plays across. */
export const MOVE = 0.2;

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

/** A stop's framing `t` of the way through its move (0 is the stop itself). */
export function moved(stop: CameraStop, t: number, out = createShot()): Shot {
  const move: CameraMove = stop.move ?? {};
  shotOf(stop, out);
  const { position, target } = out;
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
  return out;
}

interface Key {
  /** Tour progress, in stops. */
  at: number;
  shot: Shot;
  /** Whether this key sets the field of view (flight poses leave it to the stops). */
  lens: boolean;
}

/**
 * Where a stop's move keyframes sit (offset from the stop, in stops) and how
 * far through its move each is: the first stop plays forward from its
 * framing, the last one ends on it after a longer move, the rest are centred
 * on it.
 */
function windowOf(index: number, last: number): (readonly [number, number])[] {
  if (index === 0)
    return [
      [0, 0],
      [MOVE, 0.5],
    ];
  if (index === last)
    return [
      [-1.5 * MOVE, -1],
      [0, 0],
    ];
  return [
    [-MOVE, -0.5],
    [0, 0],
    [MOVE, 0.5],
  ];
}

function keysOf(stops: readonly CameraStop[]): Key[] {
  const last = stops.length - 1;
  return stops.flatMap((stop, index) => {
    const flight = (stop.via ?? []).map(via => ({
      at: index + via.at,
      shot: {
        position: new THREE.Vector3(...via.position),
        target: new THREE.Vector3(...via.target),
        fov: stop.fov,
      },
      lens: false,
    }));
    const move = windowOf(index, last).map(([offset, t]) => ({
      at: index + offset,
      shot: moved(stop, t),
      lens: true,
    }));
    return [...flight, ...move];
  });
}

/** A curve through the keys' points, timed so each point is reached at its key's progress. */
function timed(keys: readonly Key[], point: (key: Key) => THREE.Vector3) {
  const track: Track = createTrack(keys.map(point));
  const timing: Spline = createMonotone(
    keys.map(key => key.at),
    track.lengths
  );
  return (progress: number, out: THREE.Vector3) => track.pointAt(timing.at(progress), out);
}

export interface CameraPath {
  /**
   * The framing at tour progress `progress` (in stops), written into `out`.
   * With `cut` (poster capture) it is the nearest stop's framing.
   */
  shotAt(progress: number, out: Shot, cut?: boolean): Shot;
}

export function createPath(stops: readonly CameraStop[]): CameraPath {
  const last = stops.length - 1;
  const keys = keysOf(stops);
  const position = timed(keys, key => key.shot.position);
  const target = timed(keys, key => key.shot.target);
  const lenses = keys.filter(key => key.lens);
  const fov = createMonotone(
    lenses.map(key => key.at),
    lenses.map(key => key.shot.fov)
  );

  return {
    shotAt(progress, out, cut = false) {
      const first = stops[0];
      if (!first) return out;
      if (cut) return shotOf(stops[nearestStop(progress, last)] ?? first, out);
      position(progress, out.position);
      target(progress, out.target);
      out.fov = fov.at(progress);
      return out;
    },
  };
}

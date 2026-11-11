/**
 * The turn gizmo's math: which way its three rings face, how far a drag
 * round a ring turns, and the key's new turn after it. Local rings turn
 * about the axes a key is written in (the model's own, under the bone's
 * parent); World rings about the world's. Either way the key takes the turn
 * on top of the one it has, so the bone turns just as the drag shows.
 */
import * as THREE from 'three';
import type { Turn } from '@/state/recipe-keys';
import type { GizmoSpace } from '@/state/use-pose-keys-store';
import { keyTurn, turnOfQuaternion } from './recipe-keys-pose';

export type { GizmoSpace };

const AXES = [new THREE.Vector3(1, 0, 0), new THREE.Vector3(0, 1, 0), new THREE.Vector3(0, 0, 1)];

/**
 * The frame a key is written in, in the world: the parent's turn from its
 * rest (`parent`) times the model's axes (`basis`).
 */
export const keyFrame = (parent: THREE.Quaternion, basis: THREE.Quaternion) =>
  parent.clone().multiply(basis);

/** Each ring's axis in the world, X, Y and Z. */
export function ringAxes(space: GizmoSpace, frame: THREE.Quaternion): THREE.Vector3[] {
  return AXES.map(a => (space === 'local' ? a.clone().applyQuaternion(frame) : a.clone()));
}

/** Ring `index`'s axis in the key's own frame, which the key's turn is about. */
export function keyAxis(space: GizmoSpace, index: number, frame: THREE.Quaternion): THREE.Vector3 {
  const axis = AXES[index]!.clone();
  return space === 'local' ? axis : axis.applyQuaternion(frame.clone().invert()).normalize();
}

/**
 * How far a drag from `from` to `to` turns round `axis` through `center`,
 * radians, positive the right-hand way. Both points are flattened onto the
 * ring's plane first; a point on the centre turns nothing.
 */
export function ringAngle(
  center: THREE.Vector3,
  axis: THREE.Vector3,
  from: THREE.Vector3,
  to: THREE.Vector3
): number {
  const n = axis.clone().normalize();
  const flat = (p: THREE.Vector3) => {
    const v = p.clone().sub(center);
    return v.sub(n.clone().multiplyScalar(v.dot(n)));
  };
  const a = flat(from);
  const b = flat(to);
  if (a.lengthSq() < 1e-12 || b.lengthSq() < 1e-12) return 0;
  return Math.atan2(n.dot(new THREE.Vector3().crossVectors(a, b)), a.dot(b));
}

/** Where a pointer's ray meets ring `axis`'s plane through `center`, or null when it runs along it. */
export function onRingPlane(
  ray: THREE.Ray,
  center: THREE.Vector3,
  axis: THREE.Vector3
): THREE.Vector3 | null {
  const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(axis.clone().normalize(), center);
  return ray.intersectPlane(plane, new THREE.Vector3());
}

/** `turn` turned a further `angle` radians round `axis` (in the key's own frame). */
export function turnedKey(turn: Turn, axis: THREE.Vector3, angle: number): Turn {
  const step = new THREE.Quaternion().setFromAxisAngle(axis.clone().normalize(), angle);
  return turnOfQuaternion(step.multiply(keyTurn(turn)), turn);
}

/**
 * A part's holder to and from the place the assembly gives it, mirroring
 * the headless join: the pivot comes from the bounds of the model's
 * vertices, the model sits inside its holder moved so its pivot is the
 * holder's origin, and the holder stands at the place, turned about Y.
 */
import * as THREE from 'three';
import type { Pivot } from '@/ipc/client';
import type { Vec3 } from '@/state/inspector';

/** `v` to `places` decimals, never minus zero. */
const round = (v: number, places: number) => Math.round(v * 10 ** places) / 10 ** places + 0;

/** The pivot of a model loaded at its own origin, Y up. */
export function pivotPoint(model: THREE.Object3D, pivot: Pivot): THREE.Vector3 {
  model.position.set(0, 0, 0);
  model.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(model, true);
  if (box.isEmpty() || pivot === 'origin') return new THREE.Vector3();
  const mid = box.getCenter(new THREE.Vector3());
  if (pivot === 'centre') return mid;
  const x = pivot === 'left-hinge' ? box.min.x : pivot === 'right-hinge' ? box.max.x : mid.x;
  return new THREE.Vector3(x, box.min.y, mid.z);
}

/** Stands `holder` (with its model inside) where `at` and `turn` say. */
export function placeHolder(
  holder: THREE.Object3D,
  pivot: THREE.Vector3,
  at: Vec3 | null,
  turn: number
): void {
  holder.children[0]?.position.copy(pivot).negate();
  if (at) holder.position.set(...at);
  else holder.position.copy(pivot);
  holder.rotation.set(0, THREE.MathUtils.degToRad(turn), 0);
}

/**
 * Where a holder stands, to the millimetre, and how far it is turned about
 * Y, to a tenth of a degree from -180 to 180. Read from where its X axis points, so a turn past a quarter
 * does not flip into a tilt the way Euler angles would.
 */
export function placeOf(holder: THREE.Object3D): { at: Vec3; turn: number } {
  const x = new THREE.Vector3(1, 0, 0).applyQuaternion(holder.quaternion);
  const turn = THREE.MathUtils.radToDeg(Math.atan2(-x.z, x.x));
  const at = holder.position.toArray().map(v => round(v, 3)) as Vec3;
  return { at, turn: round(turn, 1) };
}

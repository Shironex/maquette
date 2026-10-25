/** A model holder's transform, to and from the layout's placement in degrees. */
import * as THREE from 'three';
import type { PrepFrame } from '@/ipc/client';
import type { Vec3 } from '@/state/inspector';
import type { PlainPlacement } from '@/state/layout';

export const deg = THREE.MathUtils.degToRad;

export function applyPlacement(holder: THREE.Object3D, p: PlainPlacement): void {
  holder.position.set(...p.position);
  holder.rotation.set(deg(p.rotation[0]), deg(p.rotation[1]), deg(p.rotation[2]), 'XYZ');
  holder.scale.setScalar(p.scale);
}

/**
 * The model inside a holder moved and scaled the way its real size and
 * origin fix-up prepare it, so the holder's placement works on top.
 */
export function applyPrep(holder: THREE.Object3D, prep: PrepFrame): void {
  const model = holder.children[0];
  if (!model) return;
  const scale = prep.scale ?? 1;
  const [x, y, z] = prep.offset.map(v => (v ?? 0) * scale);
  model.position.set(x!, y!, z!);
  model.scale.setScalar(scale);
}

export function placementFrom(holder: THREE.Object3D): PlainPlacement {
  const r = holder.rotation;
  return {
    position: holder.position.toArray() as Vec3,
    rotation: [r.x, r.y, r.z].map(THREE.MathUtils.radToDeg) as Vec3,
    scale: holder.scale.x,
  };
}

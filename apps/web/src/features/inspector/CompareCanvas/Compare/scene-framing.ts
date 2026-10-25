import * as THREE from 'three';
import type { Framing } from '@/features/viewer/studio';

/** The bounds of `group` without moving it, for a scene that keeps its layout. */
export function sceneFraming(group: THREE.Object3D): Framing | null {
  group.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(group);
  if (box.isEmpty()) return null;
  const extent = box.getSize(new THREE.Vector3()).length() || 1;
  return { box, extent, target: box.getCenter(new THREE.Vector3()) };
}

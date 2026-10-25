import type * as THREE from 'three';

/** Frees the geometry and materials of everything under `object`. */
export function disposeTree(object: THREE.Object3D): void {
  object.traverse(o => {
    const mesh = o as THREE.Mesh;
    if (!mesh.geometry) return;
    mesh.geometry.dispose();
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    materials.forEach(m => m?.dispose());
  });
}

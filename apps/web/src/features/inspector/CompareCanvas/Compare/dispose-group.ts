import type * as THREE from 'three';

/** Frees the geometry and materials of every mesh under `group`. */
export function disposeGroup(group: THREE.Object3D): void {
  group.traverse(o => {
    const mesh = o as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.geometry.dispose();
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    materials.forEach(m => m.dispose());
  });
}

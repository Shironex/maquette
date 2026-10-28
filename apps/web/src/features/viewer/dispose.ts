/**
 * Frees what three.js keeps on the GPU for loaded models and studio rigs:
 * geometry, materials and the textures the materials use. Freeing a texture
 * or geometry that a live copy still shares is safe; three uploads it again
 * on its next draw.
 */
import * as THREE from 'three';
import type { Originals } from './materials';

/** Frees `material` and every texture it holds. */
export function disposeMaterial(material: THREE.Material): void {
  for (const value of Object.values(material)) {
    if (value instanceof THREE.Texture) value.dispose();
  }
  material.dispose();
}

/** Frees the geometry, materials and textures of everything under `root`. */
export function disposeObject(root: THREE.Object3D): void {
  root.traverse(o => {
    const drawn = o as THREE.Mesh;
    if (!drawn.geometry) return;
    drawn.geometry.dispose();
    const materials = Array.isArray(drawn.material) ? drawn.material : [drawn.material];
    for (const material of materials) if (material) disposeMaterial(material);
  });
}

/** Frees a loaded model: everything under its scene, and the materials it loaded with. */
export function disposeModel(model: { scene: THREE.Object3D; originals: Originals }): void {
  disposeObject(model.scene);
  for (const loaded of model.originals.values()) {
    for (const material of Array.isArray(loaded) ? loaded : [loaded]) disposeMaterial(material);
  }
}

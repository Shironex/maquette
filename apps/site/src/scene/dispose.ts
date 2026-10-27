/**
 * Frees the GPU side of the models when the island unmounts. R3F only frees
 * what it made itself, and these come from the loader and `buildModel`. A
 * loaded file owns its geometries and textures; the clones built from it
 * share both and own only their cloned materials (and any extra geometry,
 * such as a screen plane), so a texture is freed once per file, never once
 * per clone.
 */
import type * as THREE from 'three';

type Mesh = THREE.Mesh<THREE.BufferGeometry, THREE.Material | THREE.Material[]>;

function eachMesh(root: THREE.Object3D, visit: (mesh: Mesh) => void): void {
  root.traverse(child => {
    if ((child as Mesh).isMesh) visit(child as Mesh);
  });
}

function materialsOf(mesh: Mesh): THREE.Material[] {
  return Array.isArray(mesh.material) ? mesh.material : [mesh.material];
}

/** Every texture a material holds, in any of its map slots. */
export function materialTextures(material: THREE.Material): THREE.Texture[] {
  return Object.values(material).filter(
    (value): value is THREE.Texture => (value as THREE.Texture | null)?.isTexture === true
  );
}

/** The geometries a loaded file's meshes use, which its clones share. */
export function fileGeometries(scene: THREE.Object3D): Set<THREE.BufferGeometry> {
  const found = new Set<THREE.BufferGeometry>();
  eachMesh(scene, mesh => found.add(mesh.geometry));
  return found;
}

/** Frees a loaded file: its geometries, every texture on its materials and the materials. */
export function disposeFile(scene: THREE.Object3D): void {
  const textures = new Set<THREE.Texture>();
  const materials = new Set<THREE.Material>();
  eachMesh(scene, mesh => {
    mesh.geometry.dispose();
    for (const material of materialsOf(mesh)) {
      materials.add(material);
      for (const texture of materialTextures(material)) textures.add(texture);
    }
  });
  for (const texture of textures) texture.dispose();
  for (const material of materials) material.dispose();
}

/**
 * Frees what a built model owns: its materials (the clones, the sweep's wire)
 * and any geometry that is not one of `shared`, the file's. Its textures are
 * the file's (or the shared screen textures), so they stay.
 */
export function disposeBuilt(
  root: THREE.Object3D,
  shared: ReadonlySet<THREE.BufferGeometry>
): void {
  const materials = new Set<THREE.Material>();
  const geometries = new Set<THREE.BufferGeometry>();
  eachMesh(root, mesh => {
    if (!shared.has(mesh.geometry)) geometries.add(mesh.geometry);
    for (const material of materialsOf(mesh)) materials.add(material);
  });
  for (const material of materials) material.dispose();
  for (const geometry of geometries) geometry.dispose();
}

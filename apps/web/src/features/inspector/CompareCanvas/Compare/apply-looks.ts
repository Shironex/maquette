/**
 * The live preview of material tweaks: each model of a side gets copies of
 * its materials with its asset's tweaks, the way Compress will write them.
 * A shipped build that already carries tweaks keeps its starting values in
 * `extras.maquette.original` (three.js puts extras in `userData`), so the
 * preview starts from those instead of stacking tweaks twice.
 */
import * as THREE from 'three';
import type { MaterialTweak, ModelMaterials } from '@/ipc/client';
import { dropsMap, isEmptyTweak, tweakFor } from '@/state/materials';
import type { Side } from './Compare.types';

type Standard = THREE.MeshStandardMaterial;

interface Original {
  baseColorFactor?: number[];
  roughnessFactor?: number;
  metallicFactor?: number;
  emissiveStrength?: number;
}

function isStandard(material: THREE.Material): material is Standard {
  return (material as Standard).isMeshStandardMaterial === true;
}

/** The factors the build started from, when Compress tweaked it before. */
export function originalOf(material: THREE.Material): Original | null {
  const ours = (material.userData as { maquette?: { original?: Original } }).maquette;
  return ours?.original ?? null;
}

/** Sets `material` (a copy) to `tweak` over its starting factors. */
export function tweakMaterial(material: Standard, tweak: MaterialTweak): void {
  const original = originalOf(material);
  if (original) {
    const [r = 1, g = 1, b = 1] = original.baseColorFactor ?? [];
    material.color.setRGB(r, g, b, THREE.LinearSRGBColorSpace);
    material.roughness = original.roughnessFactor ?? 1;
    material.metalness = original.metallicFactor ?? 1;
    material.emissiveIntensity = original.emissiveStrength ?? 1;
  }
  // A hex color reads as sRGB and multiplies in linear space, as Compress does.
  if (tweak.tint) material.color.multiply(new THREE.Color(tweak.tint));
  const r = tweak.roughness;
  if (r && r.value !== null) {
    material.roughness =
      r.mode === 'override' ? r.value : Math.min(1, material.roughness * r.value);
  }
  if (tweak.metalness !== null) material.metalness = tweak.metalness;
  if (dropsMap(tweak)) {
    material.roughnessMap = null;
    material.metalnessMap = null;
  }
  if (tweak.emissiveStrength !== null) material.emissiveIntensity = tweak.emissiveStrength;
  material.needsUpdate = true;
}

/**
 * Gives every model of `side` with an entry in `looks` (by asset id) its
 * tweaked materials. Returns the undo, which puts the loaded ones back.
 */
export function applyLooks(
  side: Side,
  looks: Readonly<Record<string, ModelMaterials>>
): () => void {
  const made: THREE.Material[] = [];
  const swapped: [THREE.Mesh, THREE.Material | THREE.Material[]][] = [];
  for (const holder of side.group.children) {
    const asset = holder.userData.asset as string | null | undefined;
    const look = asset ? looks[asset] : undefined;
    if (!look) continue;
    holder.traverse(obj => {
      const mesh = obj as THREE.Mesh;
      if (!mesh.isMesh) return;
      const loaded = side.originals.get(mesh) ?? mesh.material;
      const swap = (material: THREE.Material): THREE.Material => {
        const tweak = tweakFor(look, material.name);
        if (!isStandard(material) || (isEmptyTweak(tweak) && !originalOf(material))) {
          return material;
        }
        const copy = material.clone();
        tweakMaterial(copy, tweak);
        made.push(copy);
        return copy;
      };
      swapped.push([mesh, mesh.material]);
      mesh.material = Array.isArray(loaded) ? loaded.map(swap) : swap(loaded);
    });
  }
  return () => {
    for (const [mesh, material] of swapped) mesh.material = material;
    made.forEach(m => m.dispose());
  };
}

/**
 * Per model material fixes from the manifest. Every material is cloned
 * first, so a file used by two entries can be adjusted for one of them.
 */
import * as THREE from 'three';
import type { LowerBand, MaterialOverride } from './manifest';

type Standard = THREE.MeshStandardMaterial;

function isStandard(material: THREE.Material): material is Standard {
  return (material as Standard).isMeshStandardMaterial === true;
}

/** The pipeline's light bake rides on the second UV set as an emissive map. */
function isLightBake(material: Standard): boolean {
  return material.emissiveMap !== null && material.emissiveMap.channel === 1;
}

function applyBand(material: Standard, band: LowerBand, bounds: THREE.Box3): void {
  const uniforms = {
    uBandMin: { value: bounds.min.y },
    uBandMax: { value: bounds.max.y },
    uBandBelow: { value: band.below },
    uBandColor: { value: new THREE.Color(band.color) },
    uBandRoughness: { value: band.roughness },
    uBandMetalness: { value: band.metalness },
  };
  material.onBeforeCompile = shader => {
    Object.assign(shader.uniforms, uniforms);
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', '#include <common>\nvarying float vBandY;')
      .replace('#include <begin_vertex>', '#include <begin_vertex>\nvBandY = position.y;');
    shader.fragmentShader = shader.fragmentShader
      .replace(
        '#include <common>',
        `#include <common>
varying float vBandY;
uniform float uBandMin, uBandMax, uBandBelow, uBandRoughness, uBandMetalness;
uniform vec3 uBandColor;
float bandMask() {
  float h = (vBandY - uBandMin) / max(uBandMax - uBandMin, 1e-4);
  return 1.0 - smoothstep(uBandBelow - 0.015, uBandBelow + 0.015, h);
}`
      )
      .replace(
        '#include <color_fragment>',
        '#include <color_fragment>\ndiffuseColor.rgb = mix(diffuseColor.rgb, uBandColor, bandMask());'
      )
      .replace(
        '#include <metalnessmap_fragment>',
        `#include <metalnessmap_fragment>
metalnessFactor = mix(metalnessFactor, uBandMetalness, bandMask());`
      )
      .replace(
        '#include <roughnessmap_fragment>',
        `#include <roughnessmap_fragment>
roughnessFactor = mix(roughnessFactor, uBandRoughness, bandMask());`
      );
  };
  material.customProgramCacheKey = () => 'maquette-band';
}

function adjust(material: Standard, override: MaterialOverride, bounds: THREE.Box3): void {
  const baked = override.bakedLight ?? 'ao';
  if (isLightBake(material)) {
    if (baked === 'ao') {
      material.aoMap = material.emissiveMap;
      material.aoMapIntensity = 0.75;
    }
    material.emissiveMap = null;
    material.emissive.set(0x000000);
  }
  if (override.color) material.color.set(override.color);
  if (override.tint) material.color.lerp(new THREE.Color(override.tint), override.tintAmount ?? 1);
  if (override.flatSurface) {
    material.roughnessMap = null;
    material.metalnessMap = null;
  }
  if (override.roughness !== undefined) material.roughness = override.roughness;
  if (override.metalness !== undefined) material.metalness = override.metalness;
  if (override.emissiveIntensity !== undefined && material.emissiveMap) {
    material.emissiveIntensity = override.emissiveIntensity;
  }
  if (override.lower) applyBand(material, override.lower, bounds);
  material.needsUpdate = true;
}

/** Clones and adjusts every material under `root`; meshes get the shadow flags. */
export function prepareModel(
  root: THREE.Object3D,
  override: MaterialOverride = {},
  shadows: { cast: boolean; receive: boolean }
): void {
  root.traverse(child => {
    const mesh = child as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.castShadow = shadows.cast;
    mesh.receiveShadow = shadows.receive;
    mesh.geometry.computeBoundingBox();
    const bounds = mesh.geometry.boundingBox ?? new THREE.Box3();
    const swap = (material: THREE.Material) => {
      const copy = material.clone();
      if (isStandard(copy)) adjust(copy, override, bounds);
      return copy;
    };
    mesh.material = Array.isArray(mesh.material) ? mesh.material.map(swap) : swap(mesh.material);
  });
}

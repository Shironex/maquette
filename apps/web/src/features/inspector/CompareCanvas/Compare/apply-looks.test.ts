import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { originalsOf } from '@/features/viewer/materials';
import { emptyTweak } from '@/state/materials';
import { applyLooks, tweakMaterial } from './apply-looks';
import type { Side } from './Compare.types';

function side(...meshes: { asset: string | null; material: THREE.Material }[]): Side {
  const group = new THREE.Group();
  for (const m of meshes) {
    const holder = new THREE.Group();
    holder.userData.asset = m.asset;
    holder.add(new THREE.Mesh(new THREE.BoxGeometry(), m.material));
    group.add(holder);
  }
  return { group, originals: originalsOf(group), mixers: [] };
}

const meshOf = (s: Side, i: number) => s.group.children[i]!.children[0] as THREE.Mesh;

describe('the material tweak preview', () => {
  it('tints in linear space, sets the factors and drops the shared map on an override', () => {
    const m = new THREE.MeshStandardMaterial({ roughness: 0.4, metalness: 1 });
    m.roughnessMap = m.metalnessMap = new THREE.Texture();
    tweakMaterial(m, {
      ...emptyTweak(),
      tint: '#808080',
      roughness: { mode: 'override', value: 0.9 },
      emissiveStrength: 3,
    });
    expect(m.color.r).toBeCloseTo(0.2159, 3);
    expect([m.roughness, m.metalness, m.emissiveIntensity]).toEqual([0.9, 1, 3]);
    expect(m.roughnessMap).toBeNull();
  });

  it('multiplies roughness up to one and keeps the maps', () => {
    const m = new THREE.MeshStandardMaterial({ roughness: 0.6 });
    m.roughnessMap = new THREE.Texture();
    tweakMaterial(m, { ...emptyTweak(), roughness: { mode: 'multiply', value: 2 } });
    expect(m.roughness).toBe(1);
    expect(m.roughnessMap).not.toBeNull();
  });

  it('starts a tweaked build from the factors Compress recorded, not twice over', () => {
    const m = new THREE.MeshStandardMaterial({ roughness: 0.9, metalness: 0 });
    m.userData = { maquette: { original: { roughnessFactor: 0.2, metallicFactor: 1 } } };
    tweakMaterial(m, { ...emptyTweak(), roughness: { mode: 'multiply', value: 2 } });
    expect([m.roughness, m.metalness]).toEqual([0.4, 1]);
  });

  it('swaps copies onto the tweaked asset by name, and the undo puts the loaded ones back', () => {
    const lid = new THREE.MeshStandardMaterial({ name: 'Lid', metalness: 1 });
    const base = new THREE.MeshStandardMaterial({ name: 'Base', metalness: 1 });
    const other = new THREE.MeshStandardMaterial({ name: 'Lid', metalness: 1 });
    const s = side(
      { asset: 'crate', material: lid },
      { asset: 'crate', material: base },
      { asset: null, material: other }
    );
    const undo = applyLooks(s, {
      crate: { all: emptyTweak(), materials: { Lid: { ...emptyTweak(), metalness: 0 } } },
    });
    const shown = meshOf(s, 0).material as THREE.MeshStandardMaterial;
    expect(shown).not.toBe(lid);
    expect(shown.metalness).toBe(0);
    expect(lid.metalness).toBe(1);
    expect(meshOf(s, 1).material).toBe(base);
    expect(meshOf(s, 2).material).toBe(other);
    undo();
    expect(meshOf(s, 0).material).toBe(lid);
  });
});

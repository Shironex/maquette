import * as THREE from 'three';
import { describe, expect, it, vi } from 'vitest';
import { disposeModel, disposeObject } from './dispose';
import { originalsOf } from './materials';

function textured() {
  const map = new THREE.Texture();
  const material = new THREE.MeshStandardMaterial({ map });
  const mesh: THREE.Mesh = new THREE.Mesh(new THREE.BoxGeometry(), material);
  const root = new THREE.Group();
  root.add(mesh);
  return { root, mesh, material, map };
}

describe('freeing a model', () => {
  it('frees the geometry, materials and textures under a root', () => {
    const { root, mesh, material, map } = textured();
    const spies = [mesh.geometry, material, map].map(o => vi.spyOn(o, 'dispose'));
    disposeObject(root);
    for (const spy of spies) expect(spy).toHaveBeenCalledOnce();
  });

  it('frees lines too, and the materials a model loaded with after a look replaced them', () => {
    const { root, mesh, material, map } = textured();
    const line = new THREE.Line(new THREE.BufferGeometry(), new THREE.LineBasicMaterial());
    root.add(line);
    const originals = originalsOf(root);
    mesh.material = new THREE.MeshBasicMaterial();
    const spies = [material, map, line.geometry, mesh.material as THREE.Material].map(o =>
      vi.spyOn(o, 'dispose')
    );
    disposeModel({ scene: root, originals });
    for (const spy of spies) expect(spy).toHaveBeenCalled();
  });
});

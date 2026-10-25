import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { sceneFraming } from './scene-framing';

describe('framing a scene in place', () => {
  it('measures the bounds where the scene stands, without moving it', () => {
    const group = new THREE.Group();
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(2, 2, 2));
    mesh.position.set(5, 1, 0);
    group.add(mesh);
    const framing = sceneFraming(group)!;
    expect(framing.target.toArray()).toEqual([5, 1, 0]);
    expect(framing.extent).toBeCloseTo(Math.sqrt(12));
    expect(mesh.position.toArray()).toEqual([5, 1, 0]);
  });

  it('has nothing to frame in an empty scene', () => {
    expect(sceneFraming(new THREE.Group())).toBeNull();
  });
});

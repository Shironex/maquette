import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { pivotPoint, placeHolder, placeOf } from './place';

/** A door leaf 0.8 wide, 2 tall and 0.1 deep, made standing right of the origin. */
function door(): THREE.Group {
  const model = new THREE.Group();
  const leaf = new THREE.Mesh(new THREE.BoxGeometry(0.8, 2, 0.1));
  leaf.position.set(1.4, 1, 0.3);
  model.add(leaf);
  return model;
}

describe('a part in its holder', () => {
  it('finds each pivot from the bounds of the model, as the headless join does', () => {
    const at = (pivot: Parameters<typeof pivotPoint>[1]) =>
      pivotPoint(door(), pivot)
        .toArray()
        .map(v => +v.toFixed(6));
    expect(at('bottom-centre')).toEqual([1.4, 0, 0.3]);
    expect(at('left-hinge')).toEqual([1, 0, 0.3]);
    expect(at('right-hinge')).toEqual([1.8, 0, 0.3]);
    expect(at('centre')).toEqual([1.4, 1, 0.3]);
    expect(at('origin')).toEqual([0, 0, 0]);
  });

  it('stands where its model put it with no place, and turns about its pivot', () => {
    const holder = new THREE.Group();
    holder.add(door());
    const hinge = pivotPoint(holder.children[0]!, 'left-hinge');
    placeHolder(holder, hinge, null, 0);
    holder.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(holder);
    expect(box.min.x).toBeCloseTo(1);
    expect(placeOf(holder)).toEqual({ at: [1, 0, 0.3], turn: 0 });
    placeHolder(holder, hinge, [0, 0, 0], 90);
    holder.updateMatrixWorld(true);
    // A quarter turn about the hinge swings the leaf from +X to -Z.
    const swung = new THREE.Box3().setFromObject(holder);
    expect(swung.min.z).toBeCloseTo(-0.8);
    expect(swung.max.x).toBeCloseTo(0.05);
    expect(placeOf(holder)).toEqual({ at: [0, 0, 0], turn: 90 });
  });

  it('reads a turn past a quarter as a turn, not a tilt', () => {
    const holder = new THREE.Group();
    holder.rotation.set(0, THREE.MathUtils.degToRad(150), 0);
    expect(placeOf(holder).turn).toBe(150);
    holder.rotation.set(0, THREE.MathUtils.degToRad(-120), 0);
    expect(placeOf(holder).turn).toBe(-120);
    holder.position.set(0.12345, -0, 1e-5);
    expect(placeOf(holder).at).toEqual([0.123, 0, 0]);
  });
});

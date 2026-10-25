import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { applyPlacement, applyPrep, placementFrom } from './placement';

describe('model placements', () => {
  it('moves, turns in degrees and scales a holder, and reads it back', () => {
    const holder = new THREE.Group();
    applyPlacement(holder, { position: [1, 0, -2], rotation: [0, 90, 0], scale: 2 });
    expect(holder.position.toArray()).toEqual([1, 0, -2]);
    expect(holder.rotation.y).toBeCloseTo(Math.PI / 2);
    expect(holder.scale.toArray()).toEqual([2, 2, 2]);
    const back = placementFrom(holder);
    expect(back.position).toEqual([1, 0, -2]);
    expect(back.rotation[1]).toBeCloseTo(90);
    expect(back.scale).toBe(2);
  });

  it('scales and moves the model inside its holder the way the build prepares it', () => {
    const holder = new THREE.Group();
    const model = new THREE.Group();
    holder.add(model);
    applyPrep(holder, { scale: 0.5, offset: [-0.3, -0.5, 0.2] });
    expect(model.scale.toArray()).toEqual([0.5, 0.5, 0.5]);
    expect(model.position.x).toBeCloseTo(-0.15);
    expect(model.position.y).toBeCloseTo(-0.25);
    expect(model.position.z).toBeCloseTo(0.1);
    expect(holder.scale.toArray()).toEqual([1, 1, 1]);
  });
});

import { renderHook } from '@testing-library/react';
import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { useTargets } from './RecipeStage.hooks';

describe('useTargets', () => {
  it("moves the targets with the model's origin, as framing stood it on the floor", () => {
    const markers = [{ id: 'b-1', kind: 'point' as const, point: [0, 1.2, 0.4] as const }];
    const box = new THREE.Box3(new THREE.Vector3(-1, 0, -1), new THREE.Vector3(1, 1.5, 1));
    const ahead = new THREE.Vector3(0, 0, 1);
    const still = renderHook(() => useTargets(markers, box, ahead)).result.current;
    expect(still.get('b-1')?.toArray()).toEqual([0, 1.2, 0.4]);
    // A model whose bounds start a metre below its feet is lifted a metre to
    // stand on the floor; its target rises with it, so it stays where the
    // bake (which never moves the model) puts it on the model.
    const lifted = renderHook(() => useTargets(markers, box, ahead, new THREE.Vector3(0.1, 1, 0)))
      .result.current;
    const [x, y, z] = lifted.get('b-1')!.toArray();
    expect([x, y, z].map(v => Number(v.toFixed(6)))).toEqual([0.1, 2.2, 0.4]);
  });
});

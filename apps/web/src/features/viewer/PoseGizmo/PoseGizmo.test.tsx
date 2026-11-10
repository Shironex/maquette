import { act, renderHook } from '@testing-library/react';
import type { ThreeEvent } from '@react-three/fiber';
import * as THREE from 'three';
import { describe, expect, it, vi } from 'vitest';
import { step } from '@/test/fake-fiber';
import type { RecipeRig } from '../recipe-pose';
import { usePoseGizmo, type GizmoTurn } from './PoseGizmo.hooks';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

function rig(): RecipeRig {
  const chest = new THREE.Bone();
  chest.position.set(0, 1, 0);
  chest.updateWorldMatrix(true, true);
  return {
    bone: n => (n === 'chest' ? chest : undefined),
    limb: () => undefined,
    forward: new THREE.Vector3(0, 0, 1),
    facing: () => new THREE.Vector3(0, 0, 1),
    rest: () => new THREE.Quaternion(),
  };
}

/** A pointer event down a ray from above, at `x, z`. */
const press = (x: number, z: number) =>
  ({
    ray: new THREE.Ray(new THREE.Vector3(x, 5, z), new THREE.Vector3(0, -1, 0)),
    point: new THREE.Vector3(x, 1, z),
    pointerId: 1,
    stopPropagation: () => undefined,
    target: null,
  }) as unknown as ThreeEvent<PointerEvent>;

describe('the turn gizmo', () => {
  it('follows the picked bone and turns a drag round the Y ring into a key turn', () => {
    const turns: GizmoTurn[] = [];
    const dragging = vi.fn();
    const r = rig();
    const { result } = renderHook(() =>
      usePoseGizmo(r, 'chest', 'local', ['chest'], t => turns.push(t), dragging)
    );
    const group = new THREE.Group();
    (result.current.group as { current: THREE.Group | null }).current = group;
    step();
    expect(group.position.toArray()).toEqual([0, 1, 0]);

    act(() => result.current.begin(1)(press(0.1, 0)));
    act(() => result.current.move(press(0, -0.1)));
    act(() => result.current.end(press(0, -0.1)));
    expect(turns.map(t => t.phase)).toEqual(['start', 'move', 'end']);
    expect(turns[1]!.axis.toArray()).toEqual([0, 1, 0]);
    expect(THREE.MathUtils.radToDeg(turns[1]!.angle)).toBeCloseTo(90);
    expect(dragging.mock.calls).toEqual([[true], [false]]);
  });

  it('asks for nothing when the bone is not on the model', () => {
    const turns: GizmoTurn[] = [];
    const { result } = renderHook(() =>
      usePoseGizmo(rig(), 'leftHand', 'world', [], t => turns.push(t))
    );
    act(() => result.current.begin(0)(press(0.1, 0)));
    expect(turns).toEqual([]);
  });
});

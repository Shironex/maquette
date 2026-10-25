import { renderHook } from '@testing-library/react';
import * as THREE from 'three';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { resetFiber, step } from '@/test/fake-fiber';
import Skeleton from './Skeleton';
import { useSkeleton } from './Skeleton.hooks';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

/** A chain of three bones, each a metre above the last. */
function chain(): THREE.Bone[] {
  const bones = [new THREE.Bone(), new THREE.Bone(), new THREE.Bone()];
  bones[1]!.position.y = 1;
  bones[2]!.position.y = 1;
  bones[0]!.add(bones[1]!);
  bones[1]!.add(bones[2]!);
  bones[0]!.updateMatrixWorld(true);
  return bones;
}

beforeEach(() => {
  resetFiber();
});

describe('the skeleton overlay', () => {
  it('has a strut per bone with a parent bone and a joint per bone', () => {
    const { result } = renderHook(() => useSkeleton(chain(), 2));
    expect(result.current.struts.count).toBe(2);
    expect(result.current.joints.count).toBe(3);
  });

  it('follows the bones every frame', () => {
    const bones = chain();
    const { result } = renderHook(() => useSkeleton(bones, 2));
    step();
    const at = new THREE.Matrix4();
    result.current.joints.getMatrixAt(2, at);
    expect(new THREE.Vector3().setFromMatrixPosition(at).y).toBeCloseTo(2);
    bones[0]!.position.y = 5;
    bones[0]!.updateMatrixWorld(true);
    step();
    result.current.joints.getMatrixAt(2, at);
    expect(new THREE.Vector3().setFromMatrixPosition(at).y).toBeCloseTo(7);
  });

  it('frees its meshes on unmount', () => {
    const bones = chain();
    const { result, unmount } = renderHook(() => useSkeleton(bones, 2));
    const dispose = vi.spyOn(result.current.struts.geometry, 'dispose');
    unmount();
    expect(dispose).toHaveBeenCalled();
  });

  it('draws the struts and the joints', () => {
    const bones = chain();
    const { result } = renderHook(() => Skeleton({ bones, extent: 2 }));
    const parts = result.current.props.children as { props: { object: THREE.Object3D } }[];
    expect(parts.map(p => p.props.object.type)).toEqual(['Mesh', 'Mesh']);
    expect(parts.map(p => (p.props.object as THREE.InstancedMesh).isInstancedMesh)).toEqual([
      true,
      true,
    ]);
  });
});

import { renderHook } from '@testing-library/react';
import * as THREE from 'three';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { resetFiber, step } from '@/test/fake-fiber';
import Controls from './Controls';
import { useOrbitControls } from './Controls.hooks';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

describe('the preview orbit', () => {
  it('turns slowly until someone drags, and looks at the target', () => {
    const target = new THREE.Vector3(0, 1, 0);
    const { result } = renderHook(() => useOrbitControls(true, target));
    const controls = result.current;
    expect(controls.enableDamping).toBe(true);
    expect(controls.autoRotate).toBe(true);
    expect(controls.autoRotateSpeed).toBe(0.8);
    expect(controls.target.toArray()).toEqual([0, 1, 0]);
    controls.dispatchEvent({ type: 'start' });
    expect(controls.autoRotate).toBe(false);
  });

  it('updates every frame and lets go of the canvas on unmount', () => {
    const { result, unmount } = renderHook(() => useOrbitControls(false, null));
    const update = vi.spyOn(result.current, 'update');
    const dispose = vi.spyOn(result.current, 'dispose');
    step();
    step();
    expect(update).toHaveBeenCalledTimes(2);
    unmount();
    expect(dispose).toHaveBeenCalled();
  });

  it('draws nothing itself', () => {
    const { result } = renderHook(() => Controls({ turntable: false, target: null }));
    expect(result.current).toBeNull();
  });
});

import { render, renderHook, waitFor } from '@testing-library/react';
import * as THREE from 'three';
import type { VRM } from '@pixiv/three-vrm';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber, step } from '@/test/fake-fiber';
import { parseVrm } from '@/test/vrm-fixture';
import { vrmOf } from '../vrm-load';
import Scene from './Scene';
import { useIdle, useVrm } from './Scene.hooks';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

vi.mock('@/features/viewer/VrmStage/vrm-load', async importOriginal => ({
  ...(await importOriginal<typeof import('@/features/viewer/VrmStage/vrm-load')>()),
  vrmLoader: () => ({
    load(url: string, onLoad: (gltf: unknown) => void, _p: unknown, onError: (e: unknown) => void) {
      if (url.includes('bad')) queueMicrotask(() => onError(new Error('The file ends early.')));
      else void parseVrm(undefined, !url.includes('plain')).then(onLoad);
    },
  }),
}));

beforeEach(() => {
  resetFiber();
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
});
afterEach(() => vi.restoreAllMocks());

describe('the vrm scene', () => {
  it('loads the vrm, stands it on the floor, frames it and says what it holds', async () => {
    const onLoaded = vi.fn();
    const { result } = renderHook(() => useVrm('/mock/hero.vrm', onLoaded, undefined));
    await waitFor(() => expect(result.current).not.toBeNull());
    const { vrm, framing } = result.current!;
    expect(framing.box.min.y).toBeCloseTo(0);
    expect(fiber.state.camera.position.z).toBeGreaterThan(0);
    expect(fiber.state.scene.background).toBeInstanceOf(THREE.Color);
    expect(vrm.meta.metaVersion).toBe('1');
    expect(onLoaded).toHaveBeenCalledWith(
      expect.objectContaining({ spec: '1.0', missing: [], lookAt: 'expression' })
    );
  });

  it('reports a file that does not load, and a glTF with no vrm in it', async () => {
    const onError = vi.fn();
    renderHook(() => useVrm('/mock/bad.vrm', undefined, onError));
    await waitFor(() => expect(onError).toHaveBeenCalledWith('The file ends early.'));
    renderHook(() => useVrm('/mock/plain.glb', undefined, onError));
    await waitFor(() =>
      expect(onError).toHaveBeenCalledWith('This file is glTF with no VRM in it.')
    );
  });

  it('idles with the arms down and runs the vrm every frame', async () => {
    const vrm = vrmOf(await parseVrm())!;
    // The fixture has no expressions, so three-vrm makes no look at; the idle runs without one.
    expect(vrm.lookAt ?? null).toBeNull();
    const arm = vrm.humanoid.getNormalizedBoneNode('leftUpperArm')!;
    renderHook(() => useIdle(vrm, { armsDown: true, breathe: true, lookAt: true }));
    step(0.5);
    expect(arm.rotation.z).toBeLessThan(-1);
  });

  it('points the look at the pointer, and lets it go back to rest', () => {
    const lookAt = { target: null as THREE.Object3D | null, reset: vi.fn() };
    const vrm = {
      humanoid: { getNormalizedBoneNode: () => null },
      lookAt,
      update: vi.fn(),
    } as unknown as VRM;
    const { rerender, result } = renderHook(
      ({ on }) => useIdle(vrm, { armsDown: false, breathe: false, lookAt: on }),
      { initialProps: { on: true } }
    );
    expect(lookAt.target).toBe(result.current);
    const canvas = fiber.state.gl.domElement;
    canvas.dispatchEvent(new PointerEvent('pointermove', { clientX: 800, clientY: 0 }));
    step(0.1);
    expect(vrm.update).toHaveBeenCalledWith(0.1);
    const seen = result.current.position.clone().project(fiber.state.camera);
    expect(seen.x).toBeCloseTo(1);
    expect(seen.y).toBeCloseTo(1);
    rerender({ on: false });
    expect(lookAt.target).toBeNull();
    expect(lookAt.reset).toHaveBeenCalled();
  });

  it('draws the studio and the vrm once it loads', async () => {
    render(<Scene url="/mock/hero.vrm" armsDown breathe={false} lookAt />);
    await waitFor(() => expect(fiber.state.camera.position.length()).toBeGreaterThan(0.5));
  });
});

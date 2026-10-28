import { render, renderHook, waitFor } from '@testing-library/react';
import * as THREE from 'three';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber, step } from '@/test/fake-fiber';
import Scene from './Scene';
import { travelFraming, useClip, useMotionModel, useOverlays, usePoses } from './Scene.hooks';
import type { LoadedMotion } from './Scene.types';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

/** A cabinet body, a door named as a file would name it, and a walking root. */
function cabinet(): { scene: THREE.Group; animations: THREE.AnimationClip[] } {
  const scene = new THREE.Group();
  const body = new THREE.Mesh(new THREE.BoxGeometry(1, 2, 1), new THREE.MeshStandardMaterial());
  body.name = 'Body';
  const door = new THREE.Mesh(new THREE.BoxGeometry(1, 2, 0.1), new THREE.MeshStandardMaterial());
  door.name = 'Door_1';
  door.userData.name = 'Door 1';
  door.position.set(0.5, 1, 0.5);
  const hips = new THREE.Bone();
  hips.name = 'hips';
  scene.add(body, door, hips);
  const walk = new THREE.AnimationClip('Walk loop', 1, [
    new THREE.VectorKeyframeTrack('hips.position', [0, 1], [0, 0, 0, 0, 0, 2]),
  ]);
  return { scene, animations: [walk] };
}

vi.mock('@/features/viewer/studio', async importOriginal => ({
  ...(await importOriginal<typeof import('@/features/viewer/studio')>()),
  gltfLoader: () => ({
    load(url: string, onLoad: (gltf: unknown) => void, _p: unknown, onError: (e: unknown) => void) {
      queueMicrotask(() => {
        if (url.includes('bad')) onError(new Error('The file ends early.'));
        else onLoad(cabinet());
      });
    },
  }),
}));

beforeEach(() => {
  resetFiber();
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
});
afterEach(() => vi.restoreAllMocks());

async function loaded(): Promise<LoadedMotion> {
  const { result } = renderHook(() => useMotionModel('/mock/cabinet.glb', undefined));
  await waitFor(() => expect(result.current).not.toBeNull());
  return result.current!;
}

describe('the motion scene', () => {
  it('loads the model with its clips on the studio backdrop, and reports a bad file', async () => {
    const model = await loaded();
    expect(model.clips.map(c => c.name)).toEqual(['Walk loop']);
    expect(model.bones).toHaveLength(1);
    expect(fiber.state.scene.background).toBeInstanceOf(THREE.Color);
    const onError = vi.fn();
    renderHook(() => useMotionModel('/mock/bad.glb', onError));
    await waitFor(() => expect(onError).toHaveBeenCalledWith('The file ends early.'));
  });

  it('frees the model when another file replaces it', async () => {
    const { result, rerender } = renderHook(({ url }) => useMotionModel(url, undefined), {
      initialProps: { url: '/mock/cabinet.glb' },
    });
    await waitFor(() => expect(result.current).not.toBeNull());
    const body = result.current!.scene.getObjectByName('Body') as THREE.Mesh;
    const freed = vi.spyOn(body.geometry, 'dispose');
    rerender({ url: '/mock/cabinet-v2.glb' });
    await waitFor(() => expect(result.current?.scene.getObjectByName('Body')).not.toBe(body));
    expect(freed).toHaveBeenCalled();
  });

  it('poses parts by their file names and puts them back at rest', async () => {
    const model = await loaded();
    const door = model.scene.getObjectByName('Door_1')!;
    const { rerender } = renderHook(({ poses }) => usePoses(model, poses), {
      initialProps: {
        poses: { 'Door 1': { position: [0, 0.5, 0], rotation: [0, -90, 0] } } as Record<
          string,
          { position: [number, number, number]; rotation: [number, number, number] }
        > | null,
      },
    });
    expect(door.position.y).toBeCloseTo(1.5);
    expect(door.rotation.y).toBeCloseTo(-Math.PI / 2);
    rerender({ poses: null });
    expect(door.position.y).toBeCloseTo(1);
    expect(door.rotation.y).toBeCloseTo(0);
  });

  it('plays a clip at the given time, frames its walk and draws the path its root walks', async () => {
    const model = await loaded();
    const hips = model.scene.getObjectByName('hips')!;
    const framing = {
      box: new THREE.Box3(new THREE.Vector3(-0.5, 0, -0.5), new THREE.Vector3(0.5, 2, 0.5)),
      extent: 2,
      target: new THREE.Vector3(0, 1, 0),
    };
    const { result, rerender } = renderHook(
      ({ time, trail }) => useClip(model, { clip: 0, time, trail }, framing),
      { initialProps: { time: 0.5, trail: true } }
    );
    expect(hips.position.z).toBeCloseTo(1);
    expect(result.current.line).toBeInstanceOf(THREE.Line);
    expect(result.current.target?.toArray()).toEqual([0, 1, 1]);
    rerender({ time: 1, trail: false });
    expect(hips.position.z).toBeCloseTo(2);
    expect(result.current.line).toBeNull();
  });

  it('grows the framing along the walk', () => {
    const framing = {
      box: new THREE.Box3(new THREE.Vector3(-0.5, 0, -0.5), new THREE.Vector3(0.5, 2, 0.5)),
      extent: 2,
      target: new THREE.Vector3(0, 1, 0),
    };
    const grown = travelFraming(framing, [new THREE.Vector3(0, 0, 0), new THREE.Vector3(0, 0, 3)]);
    expect(grown.box.max.z).toBeCloseTo(3.5);
    expect(grown.target.toArray()).toEqual([0, 1, 1.5]);
  });

  it('lays a floor grid, outlines parts and hands each frame on', async () => {
    const model = await loaded();
    const onFrame = vi.fn();
    const { result } = renderHook(() => useOverlays(model, ['Door 1', 'Nope'], 2, onFrame));
    expect(result.current?.children.map(c => c.type)).toEqual(['GridHelper', 'BoxHelper']);
    step(0.25);
    expect(onFrame).toHaveBeenCalledWith(0.25);
  });

  it('renders without a file', () => {
    expect(() =>
      render(
        <Scene
          url={null}
          clip={null}
          time={0}
          skeleton
          trail={false}
          turntable={false}
          poses={null}
          outline={[]}
        />
      )
    ).not.toThrow();
  });
});

import { act, renderHook, waitFor } from '@testing-library/react';
import { isValidElement, type ReactElement, type ReactNode } from 'react';
import * as THREE from 'three';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetFiber } from '@/test/fake-fiber';
import { Controls } from '../Controls';
import { Model } from '../Model';
import { Skeleton } from '../Skeleton';
import Studio from './Studio';
import { useFraming, useModel, useStudioRig } from './Studio.hooks';
import type { StudioProps } from './Studio.types';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

const files = vi.hoisted(() => ({ broken: new Set<string>() }));

/** A box with a two bone chain beside it, as a skinned file would load. */
function rigged(): THREE.Group {
  const scene = new THREE.Group();
  scene.add(new THREE.Mesh(new THREE.BoxGeometry(1, 2, 1), new THREE.MeshStandardMaterial()));
  const root = new THREE.Bone();
  const tip = new THREE.Bone();
  tip.position.y = 1;
  root.add(tip);
  scene.add(root);
  return scene;
}

vi.mock('@/features/viewer/studio', async importOriginal => ({
  ...(await importOriginal<typeof import('@/features/viewer/studio')>()),
  gltfLoader: () => ({
    load(
      url: string,
      onLoad: (gltf: { scene: THREE.Group }) => void,
      _progress: unknown,
      onError: (err: unknown) => void
    ) {
      queueMicrotask(() => {
        if (files.broken.has(url)) onError(new Error(`${url} ends early.`));
        else onLoad({ scene: rigged() });
      });
    },
  }),
}));

beforeEach(() => {
  resetFiber();
  files.broken.clear();
  // jsdom has no 2D canvas; the rig's gradient textures stay blank.
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
});
afterEach(() => vi.restoreAllMocks());

describe('loading a model', () => {
  it('loads the file with its meshes and bones, and nothing without a file', async () => {
    const { result, rerender } = renderHook(({ url }) => useModel(url, undefined), {
      initialProps: { url: '/mock/a.glb' as string | null },
    });
    expect(result.current).toBeNull();
    await waitFor(() => expect(result.current?.bones).toHaveLength(2));
    expect(result.current?.originals.size).toBe(1);
    rerender({ url: null });
    expect(result.current).toBeNull();
  });

  it('reports a failed file to the latest error callback', async () => {
    files.broken.add('/mock/bad.glb');
    const first = vi.fn();
    const latest = vi.fn();
    const { rerender } = renderHook(({ onError }) => useModel('/mock/bad.glb', onError), {
      initialProps: { onError: first },
    });
    rerender({ onError: latest });
    await waitFor(() => expect(latest).toHaveBeenCalledWith('/mock/bad.glb ends early.'));
    expect(first).not.toHaveBeenCalled();
  });
});

describe('framing a model', () => {
  it('stands it on the floor, frames the camera and reports it once', async () => {
    const state = resetFiber();
    const onLoaded = vi.fn();
    const { result, rerender } = renderHook(() =>
      useFraming(useModel('/mock/a.glb', undefined), onLoaded)
    );
    await waitFor(() => expect(result.current).not.toBeNull());
    expect(result.current?.box.min.y).toBeCloseTo(0);
    expect(state.camera.position.length()).toBeGreaterThan(0);
    expect(onLoaded).toHaveBeenCalledWith({ meshes: 1, bones: 2 });
    rerender();
    expect(onLoaded).toHaveBeenCalledTimes(1);
  });

  it('builds the studio rig for a framing and disposes it when the framing goes', () => {
    const box = new THREE.Box3(new THREE.Vector3(-1, 0, -1), new THREE.Vector3(1, 2, 1));
    const framing = { box, extent: 3, target: new THREE.Vector3(0, 1, 0) };
    const { result, rerender } = renderHook(({ f }) => useStudioRig(f), {
      initialProps: { f: framing as typeof framing | null },
    });
    const rig = result.current!;
    expect(rig.name).toBe('studio');
    const floor = rig.children.find((o): o is THREE.Mesh => (o as THREE.Mesh).isMesh)!;
    const dispose = vi.spyOn(floor.geometry, 'dispose');
    rerender({ f: null });
    expect(result.current).toBeNull();
    expect(dispose).toHaveBeenCalled();
  });
});

describe('the studio', () => {
  function tree(props: StudioProps) {
    const { result } = renderHook(p => Studio(p), { initialProps: props });
    return () => {
      const root = result.current as ReactElement<{ children: ReactNode[] }>;
      return root.props.children.filter(isValidElement) as ReactElement[];
    };
  }

  it('orbits before anything has loaded', () => {
    const parts = tree({ source: { url: null }, mode: 'shaded', turntable: true });
    expect(parts().map(p => p.type)).toEqual([Controls]);
    expect(parts()[0]!.props).toEqual({ turntable: true, target: null });
  });

  it('draws the rig, the model and, in the bones look, its skeleton', async () => {
    const parts = tree({ source: { url: '/mock/a.glb' }, mode: 'bones', turntable: false });
    await waitFor(() => expect(parts()).toHaveLength(4));
    const [rig, model, skeleton, controls] = parts();
    expect(rig!.type).toBe('primitive');
    expect(model!.type).toBe(Model);
    expect(skeleton!.type).toBe(Skeleton);
    expect((skeleton!.props as { bones: THREE.Bone[] }).bones).toHaveLength(2);
    expect(controls!.type).toBe(Controls);
  });

  it('leaves the skeleton out of the other looks', async () => {
    const parts = tree({ source: { url: '/mock/a.glb' }, mode: 'shaded', turntable: false });
    await waitFor(() => expect(parts()).toHaveLength(3));
    await act(async () => {});
    expect(parts().map(p => p.type)).toEqual(['primitive', Model, Controls]);
  });
});

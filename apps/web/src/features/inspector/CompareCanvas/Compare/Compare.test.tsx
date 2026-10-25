import { fireEvent, render, waitFor } from '@testing-library/react';
import * as THREE from 'three';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetFiber, step, type FakeState } from '@/test/fake-fiber';
import type { Stop } from '@/state/inspector';
import Compare from './Compare';
import type { CompareProps } from './Compare.types';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

vi.mock('@/features/viewer/studio', async importOriginal => ({
  ...(await importOriginal<typeof import('@/features/viewer/studio')>()),
  gltfLoader: () => ({
    loadAsync: async (url: string) => {
      if (url.includes('broken')) throw new Error(`${url} ends early.`);
      const scene = new THREE.Group();
      scene.add(new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial()));
      return { scene, animations: [] };
    },
  }),
}));

const STOP: Stop = { name: 'Hero', position: [0, 2, 6], target: [0, 0, 0], fov: 40, poster: null };

let state: FakeState;
let props: CompareProps;

beforeEach(() => {
  state = resetFiber();
  // jsdom has no 2D canvas: the studio's gradients and the UV checker stay blank.
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  props = {
    kind: 'asset',
    left: [{ id: 'crate-v1', url: 'mock://v1', position: [0, 0, 0] }],
    right: [{ id: 'crate-v2', url: 'mock://v2', position: [0, 0, 0] }],
    split: 0.25,
    mode: 'shaded',
    turntable: false,
    stops: [],
    stop: null,
    handlers: {
      onSample: vi.fn(),
      onPick: vi.fn(),
      onStopShots: vi.fn(),
      onUserMove: vi.fn(),
      onLightmaps: vi.fn(),
      onError: vi.fn(),
      onReady: vi.fn(),
    },
  };
});
afterEach(() => vi.restoreAllMocks());

async function ready(): Promise<ReturnType<typeof render>> {
  const view = render(<Compare {...props} />);
  await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
  return view;
}

const renderer = () => vi.mocked(state.gl);

describe('the compare view', () => {
  it('draws the left side left of the split and the right side over the rest', async () => {
    await ready();
    step();
    expect(renderer().setScissor.mock.calls).toEqual([
      [0, 0, 200, 450],
      [200, 0, 600, 450],
    ]);
    const [leftScene, rightScene] = renderer().render.mock.calls.map(c => c[0]);
    expect(leftScene).not.toBe(rightScene);
    // Each side stands in its own studio: its rig and its models.
    expect(rightScene!.children.map(o => o.name)).toEqual(['studio', '']);
  });

  it('draws only the right side when there is nothing on the left', async () => {
    props.left = [];
    await ready();
    step();
    expect(renderer().setScissor.mock.calls).toEqual([[0, 0, 800, 450]]);
  });

  it('says when a model did not load', async () => {
    props.right = [{ id: 'gone', url: 'mock://broken', position: [0, 0, 0] }];
    render(<Compare {...props} />);
    await waitFor(() =>
      expect(props.handlers.onError).toHaveBeenCalledWith('mock://broken ends early.')
    );
  });

  it('counts the lightmapped meshes in the lightmap mode', async () => {
    props.mode = 'lightmap';
    await ready();
    expect(props.handlers.onLightmaps).toHaveBeenCalledWith(0);
  });

  it('flies to a clicked stop', async () => {
    let now = 1000;
    vi.spyOn(performance, 'now').mockImplementation(() => now);
    props.kind = 'scene';
    props.stops = [{ ...STOP, poster: 'poster.jpg' }];
    const { rerender } = await ready();
    props = { ...props, stop: { index: 0, at: 1 } };
    rerender(<Compare {...props} />);
    now += 5000;
    step();
    expect(state.camera.position.toArray()).toEqual([0, 2, 6]);
    expect(state.camera.fov).toBe(40);
  });

  it('picks the model clicked on its side', async () => {
    await ready();
    step();
    state.camera.updateMatrixWorld();
    const ndc = new THREE.Vector3(0, 0, 0).project(state.camera);
    const at = { clientX: ((ndc.x + 1) / 2) * 800, clientY: ((1 - ndc.y) / 2) * 450 };
    fireEvent.pointerDown(state.gl.domElement, at);
    fireEvent.pointerUp(state.gl.domElement, at);
    expect(props.handlers.onPick).toHaveBeenCalledWith('crate-v2');
  });

  it('reports a frame time sample every quarter second', async () => {
    let now = 0;
    vi.spyOn(performance, 'now').mockImplementation(() => (now += 4));
    await ready();
    for (let i = 0; i < 30; i += 1) step();
    expect(props.handlers.onSample).toHaveBeenCalled();
    const [ms, fps] = vi.mocked(props.handlers.onSample).mock.calls[0]!;
    expect(ms).toBeGreaterThan(0);
    expect(fps).toBeGreaterThan(0);
  });

  it('renders the stops without a poster once the scene is in place', async () => {
    props.kind = 'scene';
    props.stops = [STOP, { ...STOP, name: 'Posted', poster: 'poster.jpg' }];
    await ready();
    const draw = { drawImage: vi.fn() };
    vi.mocked(HTMLCanvasElement.prototype.getContext).mockReturnValue(
      draw as unknown as CanvasRenderingContext2D
    );
    vi.spyOn(HTMLCanvasElement.prototype, 'toDataURL').mockReturnValue('data:image/jpeg;shot');
    step();
    expect(props.handlers.onStopShots).toHaveBeenCalledWith({ 0: 'data:image/jpeg;shot' });
    expect(draw.drawImage).toHaveBeenCalledTimes(1);
    step();
    expect(props.handlers.onStopShots).toHaveBeenCalledTimes(1);
  });
});

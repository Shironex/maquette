import { fireEvent, render, waitFor } from '@testing-library/react';
import * as THREE from 'three';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { LayoutModel } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { resetFiber, step, type FakeState } from '@/test/fake-fiber';
import Editor from './Editor';
import type { EditorProps } from './Editor.types';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

vi.mock('@/features/viewer/studio', async importOriginal => ({
  ...(await importOriginal<typeof import('@/features/viewer/studio')>()),
  gltfLoader: () => ({
    loadAsync: async (url: string) => {
      if (url.includes('broken')) throw new Error(`${url} ends early.`);
      const scene = new THREE.Group();
      const box = new THREE.Mesh(
        new THREE.BoxGeometry(0.4, 0.4, 0.4),
        new THREE.MeshBasicMaterial()
      );
      box.position.y = 0.2;
      scene.add(box);
      return { scene, animations: [] };
    },
  }),
}));

function model(asset: string): LayoutModel {
  registerMockFile(`/files/${asset}.glb`, `mock://${asset}`);
  return {
    asset,
    name: asset,
    kind: 'prop',
    version: 1,
    file: `/files/${asset}.glb`,
    path: `models/${asset}.glb`,
    min: [-0.2, 0, -0.2],
    max: [0.2, 0.4, 0.2],
    fallback: [0, 0, 0],
    bytes: 1000,
    measured: 'shipped',
  } as LayoutModel;
}

const STOP = {
  id: 'hero',
  name: 'Hero',
  position: [0, 1, 3] as [number, number, number],
  target: [0, 0.2, 0] as [number, number, number],
  fov: 40,
  models: null,
};

let state: FakeState;
let props: EditorProps;

beforeEach(() => {
  state = resetFiber();
  props = {
    models: [model('crate'), model('lamp')],
    layout: {
      models: { lamp: { position: [2, 0, 0], rotation: [0, 90, 0], scale: 2 } },
      stops: [],
      desk: null,
    },
    selection: null,
    tool: 'translate',
    snap: false,
    grid: true,
    preview: null,
    frame: 0,
    handlers: {
      onSelect: vi.fn(),
      onBeginDrag: vi.fn(),
      onDragModel: vi.fn(),
      onDragStop: vi.fn(),
      onPose: vi.fn(),
      onUserMove: vi.fn(),
      onError: vi.fn(),
      onReady: vi.fn(),
    },
  };
});
afterEach(() => vi.restoreAllMocks());

/** The group the editor puts every model's holder in. */
function world(): THREE.Object3D {
  return state.scene.children.find(o => o.children.some(c => c.userData.asset))!;
}

/** Where on the canvas a point of the scene is drawn. */
function onScreen(point: THREE.Vector3): { clientX: number; clientY: number } {
  state.camera.updateMatrixWorld();
  const ndc = point.clone().project(state.camera);
  return {
    clientX: ((ndc.x + 1) / 2) * state.size.width,
    clientY: ((1 - ndc.y) / 2) * state.size.height,
  };
}

function click(at: { clientX: number; clientY: number }) {
  const el = state.gl.domElement;
  fireEvent.pointerDown(el, { ...at, pointerId: 1 });
  fireEvent.pointerUp(el, { ...at, pointerId: 1 });
}

describe('the layout editor', () => {
  it('stands every model where the layout puts it, then frames them all', async () => {
    render(<Editor {...props} />);
    await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
    const lamp = world().children.find(h => h.userData.asset === 'lamp')!;
    expect(lamp.position.toArray()).toEqual([2, 0, 0]);
    expect(lamp.rotation.y).toBeCloseTo(Math.PI / 2);
    expect(lamp.scale.x).toBe(2);
    expect(world().children).toHaveLength(2);
    expect(props.handlers.onPose).toHaveBeenCalledTimes(1);
    expect(state.camera.fov).toBe(35);
  });

  it('says how many models did not load', async () => {
    props.models = [...props.models, model('broken')];
    render(<Editor {...props} />);
    await waitFor(() =>
      expect(props.handlers.onError).toHaveBeenCalledWith('1 of 3 models did not load.')
    );
  });

  it('selects the model clicked, and nothing for a click on the backdrop', async () => {
    props.models = [model('crate')];
    render(<Editor {...props} />);
    await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
    step();
    click(onScreen(new THREE.Vector3(0, 0.2, 0)));
    expect(props.handlers.onSelect).toHaveBeenLastCalledWith({ kind: 'model', id: 'crate' });
    click({ clientX: 2, clientY: 2 });
    expect(props.handlers.onSelect).toHaveBeenLastCalledWith(null);
  });

  it('boxes the selected model and draws each stop as a camera, hidden while looking through one', async () => {
    const { rerender } = render(<Editor {...props} />);
    await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
    const box = state.scene.children.find(o => o.type === 'Box3Helper')!;
    expect(box.visible).toBe(false);
    props = { ...props, selection: { kind: 'model', id: 'lamp' } };
    props.layout = { ...props.layout, stops: [STOP] };
    rerender(<Editor {...props} />);
    expect(box.visible).toBe(true);
    const stops = state.scene.children.find(o => o.children.some(c => c.userData.stop))!;
    expect(stops.children.map(c => c.userData.stop)).toEqual(['hero']);
    expect(stops.visible).toBe(true);
    props = { ...props, preview: { id: 'hero', at: 1 } };
    rerender(<Editor {...props} />);
    expect(stops.visible).toBe(false);
  });

  it('frames every model again when the frame counter moves, and only then', async () => {
    const { rerender } = render(<Editor {...props} />);
    await waitFor(() => expect(props.handlers.onPose).toHaveBeenCalledTimes(1));
    props = { ...props, layout: { ...props.layout, desk: [3, 2] } };
    rerender(<Editor {...props} />);
    expect(props.handlers.onPose).toHaveBeenCalledTimes(1);
    props = { ...props, frame: 1 };
    rerender(<Editor {...props} />);
    expect(props.handlers.onPose).toHaveBeenCalledTimes(2);
  });

  it('flies to the stop it looks through and reports the pose when it lands', async () => {
    const now = vi.spyOn(performance, 'now').mockReturnValue(1000);
    props.layout = { ...props.layout, stops: [STOP] };
    const { rerender } = render(<Editor {...props} />);
    await waitFor(() => expect(props.handlers.onPose).toHaveBeenCalledTimes(1));
    props = { ...props, preview: { id: 'hero', at: 1 } };
    rerender(<Editor {...props} />);
    now.mockReturnValue(1000 + 5000);
    step();
    expect(state.camera.position.toArray()).toEqual([0, 1, 3]);
    expect(props.handlers.onPose).toHaveBeenLastCalledWith(
      expect.objectContaining({ position: [0, 1, 3], target: [0, 0.2, 0], fov: 40 })
    );
  });
});

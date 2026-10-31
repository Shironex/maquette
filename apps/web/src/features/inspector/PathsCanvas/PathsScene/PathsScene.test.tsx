import { fireEvent, render, waitFor } from '@testing-library/react';
import * as THREE from 'three';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { LayoutModel, ScenePath } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { resetFiber, step, type FakeState } from '@/test/fake-fiber';
import PathsScene from './PathsScene';
import type { PathsSceneProps } from './PathsScene.types';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

vi.mock('@/features/viewer/studio', async importOriginal => ({
  ...(await importOriginal<typeof import('@/features/viewer/studio')>()),
  gltfLoader: () => ({
    loadAsync: async () => {
      const scene = new THREE.Group();
      const box = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshBasicMaterial());
      box.position.y = 0.5;
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
    min: [-0.5, 0, -0.5],
    max: [0.5, 1, 0.5],
    prep: { scale: 1, offset: [0, 0, 0] },
    fallback: [0, 0, 0],
    bytes: 1000,
    measured: 'shipped',
  } as LayoutModel;
}

const CABLE: ScenePath = {
  id: 'cable',
  name: 'Cable',
  curve: 'catmull-rom',
  points: [
    [-2, 0, 2],
    [0, 0.5, 2.5],
    [2, 0, 2],
  ],
  start: 'crate',
  end: null,
  width: 0.05,
  color: '#f2c230',
  glow: '#ffe08a',
};

let state: FakeState;
let props: PathsSceneProps;

beforeEach(() => {
  state = resetFiber();
  props = {
    models: [model('crate')],
    layout: { models: {}, stops: [], desk: null },
    paths: [CABLE],
    selection: { path: 'cable', point: null },
    tool: 'select',
    snap: false,
    frame: 0,
    handlers: {
      onSelect: vi.fn(),
      onAddPoint: vi.fn(),
      onStroke: vi.fn(),
      onDragPoint: vi.fn(),
      onDropPoint: vi.fn(),
      onError: vi.fn(),
      onReady: vi.fn(),
    },
  };
});
afterEach(() => vi.restoreAllMocks());

const groupOf = (has: (o: THREE.Object3D) => boolean) =>
  state.scene.children.find(o => o.children.some(has))!;
const tubes = () => groupOf(c => c.userData.path !== undefined);
const handles = () => groupOf(c => c.userData.point !== undefined);

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

describe('the paths scene', () => {
  it('draws every path as a tube and a handle on each point of the selected one', async () => {
    const other = { ...CABLE, id: 'other', start: null, points: CABLE.points.slice(0, 2) };
    props.paths = [CABLE, other];
    render(<PathsScene {...props} />);
    await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
    expect(tubes().children.map(t => t.userData.path)).toEqual(['cable', 'other']);
    const tube = tubes().children[0] as THREE.Mesh<THREE.TubeGeometry>;
    expect(tube.geometry.parameters.radius).toBeCloseTo(0.025);
    expect(handles().children).toHaveLength(3);
    expect(handles().children[1]!.position.toArray()).toEqual([0, 0.5, 2.5]);
  });

  it('moves the handles with an edit and builds them again for a new point', async () => {
    const { rerender } = render(<PathsScene {...props} />);
    await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
    const first = handles().children[1];
    const moved: ScenePath = {
      ...CABLE,
      points: CABLE.points.map((p, i): ScenePath['points'][number] => (i === 1 ? [0, 1, 2.5] : p)),
    };
    rerender(<PathsScene {...props} paths={[moved]} />);
    expect(handles().children[1]).toBe(first);
    expect(first!.position.y).toBe(1);
    const longer: ScenePath = { ...moved, points: [...moved.points, [3, 0, 0]] };
    rerender(<PathsScene {...props} paths={[longer]} />);
    expect(handles().children).toHaveLength(4);
    expect(handles().children[1]).not.toBe(first);
  });

  it('picks the point clicked, then the path, then lets the point go', async () => {
    render(<PathsScene {...props} />);
    await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
    step();
    click(onScreen(new THREE.Vector3(0, 0.5, 2.5)));
    expect(props.handlers.onSelect).toHaveBeenLastCalledWith({ path: 'cable', point: 1 });
    click({ clientX: 2, clientY: 2 });
    expect(props.handlers.onSelect).toHaveBeenLastCalledWith({ path: 'cable', point: null });
  });

  it('adds a point where a click lands on a model or the ground', async () => {
    props.tool = 'add';
    props.snap = true;
    render(<PathsScene {...props} />);
    await waitFor(() => expect(props.handlers.onReady).toHaveBeenCalled());
    step();
    click(onScreen(new THREE.Vector3(0, 0.5, 0.5)));
    const [onModel] = vi.mocked(props.handlers.onAddPoint).mock.lastCall!;
    expect(onModel[2]).toBeCloseTo(0.5);
    click(onScreen(new THREE.Vector3(3, 0, 1)));
    const [onGround] = vi.mocked(props.handlers.onAddPoint).mock.lastCall!;
    expect(onGround[0]).toBeCloseTo(3);
    expect(onGround[1]).toBe(0);
  });
});

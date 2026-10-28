import { fireEvent, render, waitFor } from '@testing-library/react';
import * as THREE from 'three';
import type { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { registerMockFile } from '@/ipc/files';
import { resetFiber, step, type FakeState } from '@/test/fake-fiber';
import type { AssembleStageProps, StagePart } from '../AssembleStage.types';
import Placer from './Placer';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

/** Each part is a box 0.4 wide and tall, made standing 1 m along X. */
vi.mock('@/features/viewer/studio', async importOriginal => ({
  ...(await importOriginal<typeof import('@/features/viewer/studio')>()),
  gltfLoader: () => ({
    loadAsync: async (url: string) => {
      if (url.includes('broken')) throw new Error(`${url} ends early.`);
      const scene = new THREE.Group();
      const box = new THREE.Mesh(new THREE.BoxGeometry(0.4, 0.4, 0.4));
      box.position.set(1, 0.2, 0);
      scene.add(box);
      return { scene, animations: [] };
    },
  }),
}));

function part(asset: string, given: Partial<StagePart> = {}): StagePart {
  registerMockFile(`/files/${asset}.glb`, `mock://${asset}`);
  return {
    asset,
    file: `/files/${asset}.glb`,
    pivot: 'bottom-centre',
    at: null,
    turn: 0,
    ...given,
  };
}

let state: FakeState;
let props: Omit<AssembleStageProps, 'className'>;

beforeEach(() => {
  state = resetFiber();
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  props = {
    parts: [part('frame', { at: [0, 0, 0] }), part('door', { pivot: 'left-hinge' })],
    selected: 'frame',
    mode: 'translate',
    snap: { move: 0.05, turn: 15 },
    onSelect: vi.fn(),
    onPlace: vi.fn(),
    onError: vi.fn(),
  };
});
afterEach(() => vi.restoreAllMocks());

const holder = (asset: string) =>
  state.scene.children.flatMap(o => o.children).find(o => o.userData.asset === asset)!;

function gizmo(): TransformControls {
  const root = state.scene.children.find(o => 'isTransformControlsRoot' in o);
  return (root as unknown as { controls: TransformControls }).controls;
}

describe('the assembly stage', () => {
  it('stands each part at its place, and reports where a part with none stands', async () => {
    render(<Placer {...props} />);
    await waitFor(() => expect(props.onPlace).toHaveBeenCalledWith('door', [0.8, 0, 0], 0));
    expect(props.onPlace).toHaveBeenCalledTimes(1);
    expect(holder('frame').position.toArray()).toEqual([0, 0, 0]);
    expect(gizmo().object).toBe(holder('frame'));
  });

  it('puts the gizmo in its mode and snapping, and reports a drag as a place and a turn', async () => {
    const { rerender } = render(<Placer {...props} />);
    await waitFor(() => expect(props.onPlace).toHaveBeenCalled());
    const g = gizmo();
    expect(g.translationSnap).toBe(0.05);
    rerender(<Placer {...props} selected="door" mode="rotate" snap={{ move: null, turn: 45 }} />);
    expect(g.object).toBe(holder('door'));
    expect(g.getMode()).toBe('rotate');
    expect(g.showX).toBe(false);
    expect(g.rotationSnap).toBeCloseTo(Math.PI / 4);
    holder('door').rotation.y = Math.PI / 2;
    g.dispatchEvent({ type: 'objectChange' } as never);
    expect(props.onPlace).toHaveBeenLastCalledWith('door', [0.8, 0, 0], 90);
  });

  it('selects the part clicked', async () => {
    render(<Placer {...props} />);
    await waitFor(() => expect(props.onPlace).toHaveBeenCalled());
    step();
    state.camera.updateMatrixWorld();
    const ndc = new THREE.Vector3(0.2, 0.2, 0).project(state.camera);
    const at = {
      clientX: ((ndc.x + 1) / 2) * state.size.width,
      clientY: ((1 - ndc.y) / 2) * state.size.height,
      pointerId: 1,
    };
    fireEvent.pointerDown(state.gl.domElement, at);
    fireEvent.pointerUp(state.gl.domElement, at);
    expect(props.onSelect).toHaveBeenLastCalledWith('frame');
  });

  it('says how many parts did not load', async () => {
    props.parts = [...props.parts, part('broken')];
    render(<Placer {...props} />);
    await waitFor(() => expect(props.onError).toHaveBeenCalledWith('1 of 3 parts did not load.'));
  });
});

import { renderHook } from '@testing-library/react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber, step } from '@/test/fake-fiber';
import { framingAt, moverOf, useRootFollow, type RootFollow } from './root-follow';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

/** A figure whose hips sit under a root bone, standing at the origin. */
function figure() {
  const scene = new THREE.Group();
  const root = new THREE.Bone();
  root.name = 'Armature';
  const hips = new THREE.Bone();
  hips.name = 'mixamorig:Hips';
  hips.position.set(0, 1, 0);
  root.add(hips);
  scene.add(root);
  scene.updateMatrixWorld(true);
  return { scene, root, hips };
}

const framing = () => ({
  box: new THREE.Box3(new THREE.Vector3(-0.5, 0, -0.5), new THREE.Vector3(0.5, 2, 0.5)),
  extent: 2.4,
  target: new THREE.Vector3(0, 1, 0),
});

function follow(over: Partial<RootFollow> = {}) {
  const { scene, hips } = figure();
  const rig = new THREE.Group();
  const controls = new OrbitControls(fiber.state.camera, fiber.state.gl.domElement);
  const props = { scene, rig, framing: framing(), controls, follow: true, frameKey: true, ...over };
  const view = renderHook((p: RootFollow) => useRootFollow(p), { initialProps: props });
  const walk = (z: number) => {
    hips.position.z = z;
    scene.updateMatrixWorld(true);
    step();
  };
  return { ...view, props, rig, controls, walk };
}

beforeEach(() => {
  resetFiber();
});
afterEach(() => vi.restoreAllMocks());

describe('a model whose root travels', () => {
  it('moves with the hips, else the topmost bone', () => {
    const { scene, hips, root } = figure();
    expect(moverOf(scene)).toBe(hips);
    hips.name = 'Spine';
    expect(moverOf(scene)).toBe(root);
    expect(moverOf(new THREE.Group())).toBeNull();
  });

  it('keeps the floor shadow under it, and the camera with it while following', () => {
    const { rig, controls, walk } = follow();
    const camera = fiber.state.camera;
    const from = camera.position.clone();
    walk(3);
    expect(rig.position.toArray()).toEqual([0, 0, 3]);
    expect(camera.position.z - from.z).toBeCloseTo(3);
    expect(controls.target.z).toBeCloseTo(3);
  });

  it('leaves the camera where it is when not following, the shadow still under it', () => {
    const { rig, walk } = follow({ follow: false });
    const camera = fiber.state.camera;
    const from = camera.position.clone();
    walk(2);
    expect(rig.position.z).toBe(2);
    expect(camera.position.toArray()).toEqual(from.toArray());
  });

  it('frames the model again where it stands on F, and not while typing', () => {
    const { controls, walk } = follow({ follow: false });
    walk(4);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'f' }));
    expect(controls.target.toArray()).toEqual([0, 1, 4]);
    expect(fiber.state.camera.position.z).toBeGreaterThan(4);
    const field = document.createElement('input');
    document.body.append(field);
    walk(6);
    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', bubbles: true }));
    expect(controls.target.z).toBe(4);
    field.remove();
  });

  it('moves a framing along the floor only', () => {
    const moved = framingAt(framing(), new THREE.Vector3(1, 5, 2));
    expect(moved.target.toArray()).toEqual([1, 1, 2]);
    expect(moved.box.min.toArray()).toEqual([0.5, 0, 1.5]);
  });
});

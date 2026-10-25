import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { poseOf, setPose } from './camera-pose';

describe('camera poses', () => {
  it('reads where the camera stands, what it looks at and its field of view', () => {
    const camera = new THREE.PerspectiveCamera(35);
    camera.position.set(1, 2, 3);
    expect(poseOf(camera, new THREE.Vector3(0, 1, 0))).toEqual({
      position: [1, 2, 3],
      target: [0, 1, 0],
      fov: 35,
    });
  });

  it('puts the camera and the orbit target at a pose', () => {
    const camera = new THREE.PerspectiveCamera(35);
    const controls = new OrbitControls(camera, document.createElement('canvas'));
    setPose(camera, controls, { position: [0, 0, 5], target: [0, 0, 0], fov: 50 });
    expect(camera.position.toArray()).toEqual([0, 0, 5]);
    expect(controls.target.toArray()).toEqual([0, 0, 0]);
    expect(camera.fov).toBe(50);
    expect(camera.getWorldDirection(new THREE.Vector3()).z).toBeCloseTo(-1);
    controls.dispose();
  });
});

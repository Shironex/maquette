import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { applyOffsets, clipPlayer, fileName, restPoses, rootOf, rootTrail } from './motion-pose';

function figure() {
  const scene = new THREE.Group();
  const hips = new THREE.Object3D();
  hips.name = 'Hips';
  const door = new THREE.Object3D();
  door.name = 'Door_1';
  door.userData.name = 'Door 1';
  door.position.set(1, 0, 0);
  scene.add(hips, door);
  return { scene, hips, door };
}

describe('posing a model', () => {
  it('keys parts by the name the file gave them', () => {
    const { scene, door } = figure();
    expect(fileName(door)).toBe('Door 1');
    expect(restPoses(scene).get('Door 1')?.node).toBe(door);
  });

  it('moves parts by offsets from rest and puts them back', () => {
    const { scene, door } = figure();
    const parts = restPoses(scene);
    applyOffsets(parts, { 'Door 1': { position: [0, 1, 0], rotation: [0, 90, 0] } });
    expect(door.position.toArray()).toEqual([1, 1, 0]);
    expect(door.rotation.y).toBeCloseTo(Math.PI / 2);
    applyOffsets(parts, null);
    expect(door.position.toArray()).toEqual([1, 0, 0]);
    expect(door.rotation.y).toBe(0);
  });

  it('finds the root a clip walks and traces it on the floor', () => {
    const { scene } = figure();
    const track = new THREE.VectorKeyframeTrack('Hips.position', [0, 1], [0, 1, 0, 0, 1, 2]);
    const clip = new THREE.AnimationClip('Walk', 1, [track]);
    const root = rootOf(scene, clip)!;
    expect(root.name).toBe('Hips');
    const player = clipPlayer(scene, clip);
    const trail = rootTrail(player, scene, root, 1, 2);
    expect(trail.map(p => p.toArray())).toEqual([
      [0, 0, 0],
      [0, 0, 1],
      [0, 0, 2],
    ]);
    player.dispose();
  });
});

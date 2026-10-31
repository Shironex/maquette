import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { roughRetarget, type RoughPair } from './rough-retarget';

function bone(name: string, parent: THREE.Object3D | null, at: [number, number, number]) {
  const b = new THREE.Bone();
  b.name = name;
  b.position.set(...at);
  parent?.add(b);
  return b;
}

/** Hips, an upper arm, a forearm and a hand; the arm level or lowered by `drop`. */
function rig(prefix: string, hipsY: number, drop: number) {
  const root = new THREE.Group();
  const hips = bone(`${prefix}Hips`, root, [0, hipsY, 0]);
  root.add(hips);
  const arm = bone(`${prefix}Arm`, hips, [0.1, 0.4, 0]);
  const dir: [number, number, number] = [Math.cos(drop) * 0.3, -Math.sin(drop) * 0.3, 0];
  const fore = bone(`${prefix}Fore`, arm, dir);
  bone(`${prefix}Hand`, fore, dir);
  return root;
}

const PAIRS: RoughPair[] = [
  { bone: 'Hips', source: 'sHips', target: 'mHips' },
  { bone: 'LeftArm', source: 'sArm', target: 'mArm' },
  { bone: 'LeftForeArm', source: 'sFore', target: 'mFore' },
  { bone: 'LeftHand', source: 'sHand', target: 'mHand' },
];

const at = (root: THREE.Object3D, name: string) =>
  root.getObjectByName(name)!.getWorldPosition(new THREE.Vector3());

const hold = new THREE.AnimationClip('still', 1, [
  new THREE.VectorKeyframeTrack('sHips.position', [0, 1], [0, 1, 0, 1, 1, 0]),
]);

describe('the rough retarget', () => {
  it('stands an A pose model in the clip skeleton’s T pose', () => {
    const skeleton = rig('s', 1, 0);
    const model = rig('m', 1, (47 * Math.PI) / 180);
    const pose = roughRetarget(skeleton, model, hold, PAIRS);
    pose!(0);
    model.updateWorldMatrix(true, true);
    const arm = at(model, 'mFore').sub(at(model, 'mArm')).normalize();
    expect(arm.y).toBeCloseTo(0, 3);
    expect(arm.x).toBeCloseTo(1, 3);
  });

  it('moves the hips scaled by the hips height', () => {
    const skeleton = rig('s', 1, 0);
    const model = rig('m', 0.5, 0);
    const pose = roughRetarget(skeleton, model, hold, PAIRS);
    pose!(0.5);
    expect(at(model, 'mHips').x).toBeCloseTo(0.25, 3);
  });

  it('applies a turning clip bone to its model bone', () => {
    const q = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 0, 1), Math.PI / 2);
    const turn = new THREE.AnimationClip('turn', 1, [
      new THREE.QuaternionKeyframeTrack('sArm.quaternion', [0, 1], [0, 0, 0, 1, ...q.toArray()]),
    ]);
    const skeleton = rig('s', 1, 0);
    const model = rig('m', 1, 0);
    roughRetarget(skeleton, model, turn, PAIRS)!(0.5);
    model.updateWorldMatrix(true, true);
    const arm = at(model, 'mFore').sub(at(model, 'mArm')).normalize();
    expect(arm.y).toBeCloseTo(Math.SQRT1_2, 3);
  });

  it('finds bones by the names three.js gives them and gives up with none', () => {
    const skeleton = rig('s', 1, 0);
    const model = rig('m', 1, 0);
    const pairs = [{ bone: 'Hips', source: 'mixamo:Nothing', target: 'mHips' }];
    expect(roughRetarget(skeleton, model, hold, pairs)).toBeNull();
  });
});

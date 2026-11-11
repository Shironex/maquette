import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { keyTurn, layKeys, turnOfQuaternion } from './recipe-keys-pose';
import { modelBasis } from './recipe-pose';
import { keyAxis, keyFrame, onRingPlane, ringAngle, ringAxes, turnedKey } from './turn-gizmo';

const v = (x: number, y: number, z: number) => new THREE.Vector3(x, y, z);
const deg = THREE.MathUtils.radToDeg;

describe('turn gizmo math', () => {
  it('measures a drag round a ring, signed the right-hand way', () => {
    const c = v(0, 0, 0);
    expect(deg(ringAngle(c, v(0, 1, 0), v(1, 0, 0), v(0, 0, -1)))).toBeCloseTo(90);
    expect(deg(ringAngle(c, v(0, 1, 0), v(1, 5, 0), v(0, -3, 1)))).toBeCloseTo(-90);
    expect(ringAngle(c, v(0, 1, 0), v(0, 2, 0), v(1, 0, 0))).toBe(0);
  });

  it('finds where a ray meets a ring plane', () => {
    const ray = new THREE.Ray(v(0, 5, 0), v(0, -1, 0));
    expect(onRingPlane(ray, v(0, 1, 0), v(0, 1, 0))?.toArray()).toEqual([0, 1, 0]);
    expect(onRingPlane(ray, v(1, 1, 0), v(1, 0, 0))).toBeNull();
  });

  it('local rings sit on the key frame, world rings on the world', () => {
    const frame = keyFrame(new THREE.Quaternion(), modelBasis(v(1, 0, 0)));
    // A model facing +X: its X (left) is the world's -Z.
    const [x] = ringAxes('local', frame);
    expect(x!.x).toBeCloseTo(0);
    expect(x!.z).toBeCloseTo(-1);
    expect(ringAxes('world', frame)[0]!.toArray()).toEqual([1, 0, 0]);
    expect(keyAxis('local', 1, frame).toArray()).toEqual([0, 1, 0]);
    const worldX = keyAxis('world', 0, frame);
    expect(worldX.z).toBeCloseTo(1);
  });

  it('adds the drag on top of the key, and round trips a turn', () => {
    expect(turnedKey([0, 10, 0], v(0, 1, 0), THREE.MathUtils.degToRad(15))).toEqual([0, 25, 0]);
    const t = turnOfQuaternion(keyTurn([20, -30, 40]));
    expect(t).toEqual([20, -30, 40]);
  });

  it('keeps a drag past 90 degrees of Y moving on, never flipping X and Z by 180', () => {
    const past = turnedKey([0, 80, 30], v(0, 1, 0), THREE.MathUtils.degToRad(20));
    expect(past).toEqual([0, 100, 30]);
    const back = turnedKey(past, v(0, 1, 0), THREE.MathUtils.degToRad(-20));
    expect(back).toEqual([0, 80, 30]);
  });
});

describe('pose keys on the preview', () => {
  function rig() {
    const root = new THREE.Bone();
    const arm = new THREE.Bone();
    arm.position.set(0, 1, 0);
    root.add(arm);
    root.updateWorldMatrix(true, true);
    const rest = new Map<THREE.Object3D, THREE.Quaternion>([
      [root, new THREE.Quaternion()],
      [arm, new THREE.Quaternion()],
    ]);
    const bones: Record<string, THREE.Object3D> = { chest: root, leftUpperArm: arm };
    return {
      root,
      arm,
      keyRig: {
        bone: (n: string) => bones[n],
        forward: v(0, 0, 1),
        rest: (b: THREE.Object3D) => rest.get(b),
      },
    };
  }
  const angle = (q: THREE.Quaternion) => deg(2 * Math.acos(Math.min(1, Math.abs(q.w))));

  it('adds a turn on top, and a child turns with its keyed parent', () => {
    const { root, arm, keyRig } = rig();
    arm.quaternion.setFromAxisAngle(v(0, 0, 1), THREE.MathUtils.degToRad(10));
    arm.updateWorldMatrix(false, true);
    layKeys(
      keyRig,
      new Map([
        ['chest', [0, 30, 0] as [number, number, number]],
        ['leftUpperArm', [0, 0, 20] as [number, number, number]],
      ]),
      'add',
      1
    );
    expect(angle(root.quaternion)).toBeCloseTo(30);
    // Under its parent, the arm has its own 10 and the key's 20.
    expect(angle(arm.quaternion)).toBeCloseTo(30);
  });

  it('replaces from the rest pose, and weighs it', () => {
    const { arm, keyRig } = rig();
    arm.quaternion.setFromAxisAngle(v(1, 0, 0), THREE.MathUtils.degToRad(50));
    arm.updateWorldMatrix(false, true);
    layKeys(
      keyRig,
      new Map([['leftUpperArm', [0, 0, 40] as [number, number, number]]]),
      'replace',
      1
    );
    const want = keyTurn([0, 0, 40]);
    expect(angle(arm.quaternion.clone().invert().multiply(want))).toBeCloseTo(0);
    arm.quaternion.identity();
    arm.updateWorldMatrix(false, true);
    layKeys(
      keyRig,
      new Map([['leftUpperArm', [0, 0, 40] as [number, number, number]]]),
      'replace',
      0.5
    );
    expect(angle(arm.quaternion)).toBeCloseTo(20);
  });
});

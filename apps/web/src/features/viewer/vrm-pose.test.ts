import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { breathAt, poseHumanoid, type PosableHumanoid } from './vrm-pose';

/** A humanoid of named nodes at rest, as normalised bones are. */
function humanoid(names: string[]): PosableHumanoid & { nodes: Map<string, THREE.Object3D> } {
  const nodes = new Map(names.map(n => [n, new THREE.Object3D()]));
  return { nodes, getNormalizedBoneNode: (name: never) => nodes.get(name) ?? null };
}

const ARM = new THREE.Vector3(1, 0, 0);
const BONES = ['leftUpperArm', 'rightUpperArm', 'leftLowerArm', 'rightLowerArm', 'chest', 'spine'];

describe('posing a vrm humanoid', () => {
  it('brings both arms down from level and the elbows forward', () => {
    const h = humanoid(BONES);
    poseHumanoid(h, 0, { armsDown: true, breathe: false });
    const left = ARM.clone().applyEuler(h.nodes.get('leftUpperArm')!.rotation);
    const right = ARM.clone().negate().applyEuler(h.nodes.get('rightUpperArm')!.rotation);
    expect(left.y).toBeLessThan(-0.9);
    expect(left.x).toBeGreaterThan(0);
    expect(right.y).toBeLessThan(-0.9);
    expect(right.x).toBeLessThan(0);
    const forearm = ARM.clone().applyEuler(h.nodes.get('leftLowerArm')!.rotation);
    expect(forearm.z).toBeGreaterThan(0);
  });

  it('puts every bone back to rest with both options off', () => {
    const h = humanoid(BONES);
    poseHumanoid(h, 1, { armsDown: true, breathe: true });
    poseHumanoid(h, 1, { armsDown: false, breathe: false });
    for (const node of h.nodes.values()) {
      expect(node.quaternion.angleTo(new THREE.Quaternion())).toBeCloseTo(0);
    }
  });

  it('breathes in the chest, falling back to the spine', () => {
    const h = humanoid(BONES.filter(b => b !== 'chest'));
    poseHumanoid(h, 2.1, { armsDown: false, breathe: true });
    expect(h.nodes.get('spine')!.rotation.x).toBeLessThan(0);
    expect(breathAt(0)).toBe(0);
    expect(breathAt(2.1)).toBeCloseTo(1);
  });

  it('skips the bones a rig does not have', () => {
    expect(() => poseHumanoid(humanoid([]), 0, { armsDown: true, breathe: true })).not.toThrow();
  });
});

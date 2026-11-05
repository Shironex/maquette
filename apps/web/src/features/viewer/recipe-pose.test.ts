import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import type { RecipeLayer } from '@/ipc/client';
import { checkReaches } from './check-reaches';
import { envelope, poseRecipe, solveTwoBone, targetsOf, type RecipeRig } from './recipe-pose';

const v = (x: number, y: number, z: number) => new THREE.Vector3(x, y, z);

/** A right arm out along +X with a slight bend back at the elbow, on a root at the origin. */
function arm() {
  const root = new THREE.Bone();
  const head = new THREE.Bone();
  const upper = new THREE.Bone();
  const lower = new THREE.Bone();
  const hand = new THREE.Bone();
  head.position.set(0, 1.6, 0);
  upper.position.set(0, 1.4, 0);
  lower.position.set(0.3, 0, -0.05);
  hand.position.set(0.3, 0, 0.05);
  root.add(head, upper);
  upper.add(lower);
  lower.add(hand);
  root.updateWorldMatrix(true, true);
  const bones: Record<string, THREE.Object3D> = {
    rightHand: hand,
    rightUpperArm: upper,
    rightLowerArm: lower,
    head,
  };
  const rest = [root, head, upper, lower, hand].map(
    b => [b, b.position.clone(), b.quaternion.clone()] as const
  );
  const reset = () => {
    for (const [b, p, q] of rest) {
      b.position.copy(p);
      b.quaternion.copy(q);
    }
    root.updateWorldMatrix(true, true);
  };
  const rig: RecipeRig = {
    bone: name => bones[name],
    limb: end => (end === 'rightHand' ? { upper, lower } : undefined),
    forward: v(0, 0, 1),
    facing: () => v(0, 0, 1),
  };
  return { root, hand, lower, upper, head, rig, reset };
}

const at = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());

const layer = (
  over: Partial<RecipeLayer> & Pick<RecipeLayer, 'kind' | 'bone'>,
  s = {}
): RecipeLayer => ({
  id: 'l-1',
  ...over,
  settings: {
    weight: 1,
    start: 0,
    end: 1,
    target: null,
    rotation: null,
    offset: null,
    cycles: null,
    amount: null,
    ...s,
  },
});

describe('the recipe pose', () => {
  it('eases a change in and out over its range, and a reach in and holding to its end', () => {
    expect(envelope('nudge', 0.2, 0.6, 0.1)).toBe(0);
    expect(envelope('nudge', 0.2, 0.6, 0.4)).toBe(1);
    expect(envelope('nudge', 0.2, 0.6, 0.6)).toBe(0);
    expect(envelope('reach', 0.2, 0.6, 0.6)).toBe(1);
    expect(envelope('reach', 0.2, 0.6, 0.3)).toBeCloseTo(0.5);
    expect(envelope('wave', 0.5, 0.5, 0.5)).toBe(0);
  });

  it('solves a two-bone limb onto a target it reaches, keeping lengths and the bend side', () => {
    const root = v(0, 0, 0);
    const mid = v(0.3, 0, -0.05);
    const end = v(0.6, 0, 0);
    const target = v(0.3, 0.3, 0.1);
    const solve = solveTwoBone(root, mid, end, target);
    expect(solve.reached).toBe(true);
    expect(solve.end.distanceTo(target)).toBeLessThan(1e-6);
    expect(solve.mid.distanceTo(root)).toBeCloseTo(mid.distanceTo(root), 6);
    expect(solve.end.distanceTo(solve.mid)).toBeCloseTo(end.distanceTo(mid), 6);
  });

  it('stretches toward a target past its reach and says it did not reach', () => {
    const solve = solveTwoBone(v(0, 0, 0), v(0.3, 0, -0.05), v(0.6, 0, 0), v(0, 5, 0));
    expect(solve.reached).toBe(false);
    expect(solve.end.x).toBeCloseTo(0, 6);
    expect(solve.end.y).toBeGreaterThan(0.59);
  });

  it('lays a reach over the pose so the hand meets its blank at the reach end', () => {
    const { hand, rig } = arm();
    const target = v(0.35, 1.7, 0.2);
    const reach = layer(
      { kind: 'reach', bone: 'rightHand' },
      { start: 0.2, end: 0.6, target: 'lantern' }
    );
    poseRecipe(rig, [reach], new Map([['lantern', target]]), 0.6);
    expect(at(hand).distanceTo(target)).toBeLessThan(1e-4);
  });

  it('moves a nudged bone by its offset in metres and turns a waved arm', () => {
    const { hand, lower, rig } = arm();
    const before = at(hand);
    const nudge = layer(
      { kind: 'nudge', bone: 'rightHand' },
      { offset: [0, 0.1, 0], start: 0, end: 1 }
    );
    poseRecipe(rig, [nudge], new Map(), 0.5);
    expect(at(hand).sub(before).y).toBeCloseTo(0.1, 5);
    const turned = lower.quaternion.clone();
    const wave = layer(
      { kind: 'wave', bone: 'rightLowerArm' },
      { cycles: 1, amount: 30, start: 0, end: 1 }
    );
    poseRecipe(rig, [wave], new Map(), 0.4);
    expect(lower.quaternion.angleTo(turned)).toBeGreaterThan(0.1);
  });

  it('places a point where it says and props in slots in front of the model', () => {
    const targets = targetsOf(
      [
        { id: 'shelf', kind: 'point', point: [0.2, 1.1, 0.4] },
        { id: 'lantern', kind: 'prop', point: null },
        { id: 'cup', kind: 'prop', point: null },
      ],
      2
    );
    expect(targets.get('shelf')!.toArray()).toEqual([0.2, 1.1, 0.4]);
    expect(targets.get('lantern')!.toArray()).toEqual([-0.44, 1.3, 0.36]);
    expect(targets.get('cup')!.x).toBeCloseTo(0.44);
  });
});

describe('the live checks', () => {
  it('measures the gap at a reach end, and none for a target in reach', () => {
    const { rig, reset } = arm();
    const reach = layer(
      { kind: 'reach', bone: 'rightHand' },
      { start: 0.2, end: 0.6, target: 'near' }
    );
    const near = checkReaches(reset, rig, [reach], new Map([['near', v(0.35, 1.7, 0.2)]]));
    expect(near).toHaveLength(1);
    expect(near[0]!.gap).toBeLessThan(0.001);
    expect(near[0]!.flipped).toBe(false);
    const far = checkReaches(reset, rig, [reach], new Map([['near', v(3, 1.4, 0)]]));
    expect(far[0]!.gap).toBeGreaterThan(2);
  });

  it('skips a reach whose blank is open', () => {
    const { rig, reset } = arm();
    const reach = layer({ kind: 'reach', bone: 'rightHand' }, { target: 'open' });
    expect(checkReaches(reset, rig, [reach], new Map())).toEqual([]);
  });
});

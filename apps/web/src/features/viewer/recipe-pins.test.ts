import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import type { RecipeLayer } from '@/ipc/client';
import type { PinSpan } from '@/state/recipe-pins';
import { checkPins, holdPins, pinSpots, type PoseAt } from './recipe-pins';
import type { RecipeRig } from './recipe-pose';

const v = (x: number, y: number, z: number) => new THREE.Vector3(x, y, z);
const at = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());

/**
 * A hips bone with a left leg hanging down, knee a little forward, and a
 * foot pointing ahead. The "clip" slides the hips `slide` metres along X
 * per frame from frame 0, so an unpinned foot slides with them.
 */
function leg(slide = 0.01) {
  const root = new THREE.Bone();
  const hips = new THREE.Bone();
  const upper = new THREE.Bone();
  const lower = new THREE.Bone();
  const foot = new THREE.Bone();
  hips.position.set(0, 1, 0);
  upper.position.set(0.1, -0.05, 0);
  lower.position.set(0, -0.45, 0.1);
  foot.position.set(0, -0.45, -0.1);
  root.add(hips);
  hips.add(upper);
  upper.add(lower);
  lower.add(foot);
  const rest = [hips, upper, lower, foot].map(
    b => [b, b.position.clone(), b.quaternion.clone()] as const
  );
  const bones: Record<string, THREE.Object3D> = {
    hips,
    leftFoot: foot,
    leftUpperLeg: upper,
    leftLowerLeg: lower,
  };
  const rig: RecipeRig = {
    bone: name => bones[name],
    limb: end => (end === 'leftFoot' ? { upper, lower } : undefined),
    forward: v(0, 0, 1),
    facing: () => v(0, 0, 1),
  };
  const poseAt: PoseAt = frame => {
    for (const [b, p, q] of rest) {
      b.position.copy(p);
      b.quaternion.copy(q);
    }
    hips.position.x += slide * frame;
    // The clip also turns the foot a little each frame.
    foot.quaternion.setFromAxisAngle(v(0, 1, 0), 0.02 * frame);
    root.updateWorldMatrix(true, true);
    return frame / 40;
  };
  return { rig, poseAt, foot, hips, upper };
}

const pinLayer: RecipeLayer = {
  id: 'l-1',
  kind: 'pin',
  bone: 'leftFoot',
  settings: {
    weight: 1,
    start: 0.25,
    end: 0.75,
    target: null,
    rotation: null,
    offset: null,
    cycles: null,
    amount: null,
  },
};

const span = (over: Partial<PinSpan> = {}): PinSpan => ({
  layer: 'l-1',
  bone: 'leftFoot',
  first: 10,
  last: 30,
  easeIn: 3,
  easeOut: 3,
  weight: 1,
  flat: 'toe-and-heel',
  ...over,
});

describe('pins on the rough preview', () => {
  it('takes the spot where the foot is at its first frame and holds it there', () => {
    const l = leg();
    const spans = [span()];
    const spots = pinSpots(l.poseAt, l.rig, [pinLayer], new Map(), spans);
    l.poseAt(10);
    const spot = at(l.foot);
    expect(spots.get('l-1')?.position.distanceTo(spot)).toBeLessThan(1e-9);
    for (const frame of [13, 20, 27]) {
      l.poseAt(frame);
      const slid = at(l.foot).distanceTo(spot);
      expect(slid).toBeGreaterThan(0.02);
      holdPins(l.rig, spans, spots, frame);
      expect(at(l.foot).distanceTo(spot), `frame ${frame}`).toBeLessThan(1e-4);
    }
  });

  it('keeps toe and heel flat: the foot keeps the turn it had at its first frame', () => {
    const l = leg();
    const spans = [span()];
    const spots = pinSpots(l.poseAt, l.rig, [pinLayer], new Map(), spans);
    const turn = spots.get('l-1')!.rotation;
    l.poseAt(20);
    holdPins(l.rig, spans, spots, 20);
    const now = l.foot.getWorldQuaternion(new THREE.Quaternion());
    expect(now.angleTo(turn)).toBeLessThan(1e-4);
  });

  it('lets the foot turn as the clip has it when keep flat is off', () => {
    const l = leg();
    const spans = [span({ flat: 'off' })];
    const spots = pinSpots(l.poseAt, l.rig, [pinLayer], new Map(), spans);
    const turn = spots.get('l-1')!.rotation;
    l.poseAt(20);
    holdPins(l.rig, spans, spots, 20);
    const now = l.foot.getWorldQuaternion(new THREE.Quaternion());
    expect(now.angleTo(turn)).toBeGreaterThan(0.05);
  });

  it('eases in from its first frame and lets go outside its frames', () => {
    const l = leg();
    const spans = [span()];
    const spots = pinSpots(l.poseAt, l.rig, [pinLayer], new Map(), spans);
    const spot = spots.get('l-1')!.position;
    l.poseAt(35);
    const free = at(l.foot).clone();
    holdPins(l.rig, spans, spots, 35);
    expect(at(l.foot).distanceTo(free)).toBeLessThan(1e-9);
    l.poseAt(11);
    const before = at(l.foot).distanceTo(spot);
    holdPins(l.rig, spans, spots, 11);
    const eased = at(l.foot).distanceTo(spot);
    expect(eased).toBeLessThan(before);
    expect(eased).toBeGreaterThan(1e-4);
  });

  it('stays on a blank target instead, and says when the leg cannot reach it', () => {
    const l = leg(0.03);
    const target = v(0.1, 0.1, 0.1);
    const spans = [span({ target: 'spot' })];
    const spots = pinSpots(l.poseAt, l.rig, [pinLayer], new Map([['spot', target]]), spans);
    expect(spots.get('l-1')?.position.distanceTo(target)).toBeLessThan(1e-9);
    const [check] = checkPins(l.poseAt, l.rig, [pinLayer], new Map(), spans, spots);
    expect(check?.limb).toBeCloseTo(0.9, 1);
    expect(check?.hips).toBeCloseTo(0.6, 2);
    expect(check?.outFrame).not.toBeNull();
  });

  it('can stay on its spot when the hips move little', () => {
    const l = leg(0.002);
    const spans = [span()];
    const spots = pinSpots(l.poseAt, l.rig, [pinLayer], new Map(), spans);
    const [check] = checkPins(l.poseAt, l.rig, [pinLayer], new Map(), spans, spots);
    expect(check?.outFrame).toBeNull();
    expect(check?.hips).toBeCloseTo(0.04, 3);
  });
});

import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import type { RecipeBase, RecipeFile } from '@/ipc/client';
import { timeline } from '@/state/recipe-timeline';
import { joinComposer, reflectPoint, reflectRotation, type JoinPoser } from './recipe-join';

const clip = (name: string, frames: number, more: Partial<RecipeBase> = {}): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: false,
  ...more,
});

const recipe = (more: Partial<RecipeFile> = {}): RecipeFile => ({
  version: 2,
  name: 'Two',
  revision: 1,
  created: '',
  edited: '',
  base: clip('A', 30),
  joined: [clip('B', 30, { blend: { frames: 0, lineUp: 'hips' } })],
  blanks: [],
  layers: [],
  from: null,
  ...more,
});

/** Hips, spine, chest and two arms; clip A walks the hips along +X, clip B stands at -5. */
function rig() {
  const root = new THREE.Group();
  const hips = new THREE.Bone();
  const spine = new THREE.Bone();
  const chest = new THREE.Bone();
  const leftArm = new THREE.Bone();
  const rightArm = new THREE.Bone();
  hips.position.set(0, 1, 0);
  spine.position.set(0, 0.2, 0);
  chest.position.set(0, 0.2, 0);
  leftArm.position.set(0.2, 0.1, 0);
  rightArm.position.set(-0.2, 0.1, 0);
  root.add(hips);
  hips.add(spine);
  spine.add(chest);
  chest.add(leftArm, rightArm);
  root.updateWorldMatrix(true, true);
  const bones = [hips, spine, chest, leftArm, rightArm];
  const rest = bones.map(b => [b.position.clone(), b.quaternion.clone()] as const);
  const poser: JoinPoser = {
    bones,
    hips,
    partner: b => (b === leftArm ? rightArm : b === rightArm ? leftArm : b),
    chest,
    spine,
    forward: new THREE.Vector3(0, 0, 1),
    origin: new THREE.Vector3(),
    rest: () => {
      bones.forEach((b, i) => {
        b.position.copy(rest[i]![0]);
        b.quaternion.copy(rest[i]![1]);
      });
      root.updateWorldMatrix(true, true);
    },
    put: (c, at) => {
      if (c === 0) hips.position.set(at / 10, 1, 0);
      else hips.position.set(-5, 1, 0);
      leftArm.quaternion.setFromAxisAngle(new THREE.Vector3(0, 0, 1), c === 0 ? 0.5 : 0);
      root.updateWorldMatrix(true, true);
    },
  };
  return { root, hips, chest, leftArm, rightArm, poser };
}

const at = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());

describe('joined clips on a model', () => {
  it('lines the next clip up over the hips of the one before at the seam', () => {
    const r = rig();
    const t = timeline(recipe());
    const play = joinComposer(r.poser, t);
    const seam = t.seams[0]!;
    play.poseFrame(seam.frame - 1);
    const before = at(r.hips);
    play.poseFrame(seam.frame + 2);
    expect(at(r.hips).x).toBeCloseTo(3, 5);
    expect(before.x).toBeLessThanOrEqual(3);
  });

  it('starts a clip where it was made with the line up off', () => {
    const r = rig();
    const t = timeline(
      recipe({ joined: [clip('B', 30, { blend: { frames: 0, lineUp: 'off' } })] })
    );
    joinComposer(r.poser, t).poseFrame(t.frames);
    expect(at(r.hips).x).toBeCloseTo(-5, 5);
  });

  it('swaps the sides and flips across the middle with the mirror on', () => {
    const r = rig();
    const t = timeline(recipe({ joined: [], mirror: true }));
    joinComposer(r.poser, t).poseFrame(20);
    expect(at(r.hips).x).toBeCloseTo(-2, 5);
    const right = r.rightArm.getWorldQuaternion(new THREE.Quaternion());
    const angle = 2 * Math.acos(Math.min(Math.abs(right.w), 1));
    expect(angle).toBeCloseTo(0.5, 4);
    expect(right.z).toBeLessThan(0);
  });

  it('breathes on the chest while a pose holds', () => {
    const r = rig();
    const t = timeline(
      recipe({ joined: [], time: [{ kind: 'freeze', clip: 0, at: 10, frames: 60, breathe: 1 }] })
    );
    const play = joinComposer(r.poser, t);
    play.poseFrame(32);
    const turned = r.chest.quaternion.angleTo(new THREE.Quaternion());
    expect(turned).toBeGreaterThan(0.01);
  });

  it('reflects turns and points across a plane', () => {
    const x = new THREE.Vector3(1, 0, 0);
    const q = new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), 0.4);
    const r = reflectRotation(q, x);
    expect(r.y).toBeCloseTo(-q.y, 6);
    expect(reflectPoint(new THREE.Vector3(2, 1, 0), new THREE.Vector3(), x).x).toBe(-2);
  });
});

import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { parseVrm } from '@/test/vrm-fixture';
import { poseHumanoid, REQUIRED_BONES } from '../vrm-pose';
import { summarize, vrmOf } from './vrm-load';

describe('a written vrm read back through three-vrm', () => {
  it('holds a humanoid with every required bone, and says what it carries', async () => {
    const gltf = await parseVrm();
    const vrm = vrmOf(gltf);
    expect(vrm).not.toBeNull();
    for (const bone of REQUIRED_BONES) expect(vrm!.humanoid.getRawBoneNode(bone)).not.toBeNull();
    expect(summarize(gltf, vrm!)).toEqual({
      spec: '1.0',
      bones: 52,
      missing: [],
      springs: 0,
      expressions: 0,
      lookAt: 'expression',
      lookAtMoves: false,
    });
  });

  it('lowers the real hands when the normalised arms come down', async () => {
    const vrm = vrmOf(await parseVrm())!;
    vrm.scene.updateMatrixWorld(true);
    const at = (bone: 'leftHand' | 'leftUpperArm') =>
      vrm.humanoid.getRawBoneNode(bone)!.getWorldPosition(new THREE.Vector3());
    const shoulder = at('leftUpperArm');
    expect(at('leftHand').y).toBeCloseTo(shoulder.y, 1);
    poseHumanoid(vrm.humanoid, 0, { armsDown: true, breathe: false });
    vrm.update(0);
    vrm.scene.updateMatrixWorld(true);
    const hand = at('leftHand');
    expect(hand.y).toBeLessThan(shoulder.y - 0.2);
    expect(hand.x).toBeGreaterThan(0);
  });

  it('finds no VRM in a plain glTF', async () => {
    expect(vrmOf(await parseVrm(undefined, false))).toBeNull();
  });
});

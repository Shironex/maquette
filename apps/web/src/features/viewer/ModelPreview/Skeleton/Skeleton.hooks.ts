import { useEffect, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { CLAY } from '../../studio';

export interface SkeletonMeshes {
  /** A tapered strut from each bone to its parent. */
  struts: THREE.InstancedMesh;
  /** A ball per joint. */
  joints: THREE.InstancedMesh;
}

/**
 * The instanced struts and joints for `bones`, disposed with them, and
 * re-posed every frame so they follow animation.
 */
export function useSkeleton(bones: THREE.Bone[], extent: number): SkeletonMeshes {
  const { struts, joints, pairs } = useMemo(() => {
    const pairs = bones.filter(b => (b.parent as THREE.Bone | null)?.isBone);
    const material = new THREE.MeshBasicMaterial({
      color: CLAY,
      depthTest: false,
      transparent: true,
      opacity: 0.95,
    });
    const jointMaterial = new THREE.MeshBasicMaterial({
      color: '#f3c9b3',
      depthTest: false,
      transparent: true,
    });
    // A unit strut along +Y from 0 to 1, wide at the parent end.
    const strut = new THREE.CylinderGeometry(0.35, 1, 1, 6, 1);
    strut.translate(0, 0.5, 0);
    const struts = new THREE.InstancedMesh(strut, material, Math.max(pairs.length, 1));
    const joints = new THREE.InstancedMesh(
      new THREE.SphereGeometry(1, 10, 8),
      jointMaterial,
      Math.max(bones.length, 1)
    );
    struts.count = pairs.length;
    joints.count = bones.length;
    struts.renderOrder = 10;
    joints.renderOrder = 11;
    struts.frustumCulled = false;
    joints.frustumCulled = false;
    return { struts, joints, pairs };
  }, [bones]);
  useEffect(
    () => () => {
      for (const o of [struts, joints]) {
        o.geometry.dispose();
        (o.material as THREE.Material).dispose();
        o.dispose();
      }
    },
    [struts, joints]
  );
  const scratch = useMemo(
    () => ({
      from: new THREE.Vector3(),
      to: new THREE.Vector3(),
      dir: new THREE.Vector3(),
      up: new THREE.Vector3(0, 1, 0),
      turn: new THREE.Quaternion(),
      size: new THREE.Vector3(),
      matrix: new THREE.Matrix4(),
    }),
    []
  );
  useFrame(() => {
    const { from, to, dir, up, turn, size, matrix } = scratch;
    const width = extent * 0.006;
    pairs.forEach((bone, i) => {
      (bone.parent as THREE.Object3D).getWorldPosition(from);
      bone.getWorldPosition(to);
      dir.subVectors(to, from);
      const length = dir.length();
      if (length > 1e-6) turn.setFromUnitVectors(up, dir.divideScalar(length));
      size.set(width, length, width);
      matrix.compose(from, turn, size);
      struts.setMatrixAt(i, matrix);
    });
    struts.instanceMatrix.needsUpdate = true;
    const r = extent * 0.007;
    bones.forEach((bone, i) => {
      bone.getWorldPosition(from);
      matrix.compose(from, turn.identity(), size.set(r, r, r));
      joints.setMatrixAt(i, matrix);
    });
    joints.instanceMatrix.needsUpdate = true;
  });
  return { struts, joints };
}

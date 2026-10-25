import type * as THREE from 'three';
import { useSkeleton } from './Skeleton.hooks';

export interface SkeletonProps {
  bones: THREE.Bone[];
  extent: number;
}

/**
 * The skeleton in clay: a tapered strut from each bone to its parent and a
 * ball per joint, drawn over the ghosted mesh. Sized to the model so it reads
 * at any scale, and re-posed every frame so it follows animation.
 */
export default function Skeleton({ bones, extent }: SkeletonProps) {
  const { struts, joints } = useSkeleton(bones, extent);
  return (
    <>
      <primitive object={struts} />
      <primitive object={joints} />
    </>
  );
}

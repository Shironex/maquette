import type * as THREE from 'three';
import type { Originals } from '../../materials';
import type { MotionStageProps } from '../MotionStage.types';

export type SceneProps = Omit<MotionStageProps, 'className' | 'label'>;

/** A loaded model with its clips and its named nodes at rest. */
export interface LoadedMotion {
  scene: THREE.Group;
  originals: Originals;
  bones: THREE.Bone[];
  clips: THREE.AnimationClip[];
}

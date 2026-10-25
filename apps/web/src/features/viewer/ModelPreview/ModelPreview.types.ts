import type * as THREE from 'three';
import type { Originals } from '../materials';

export type PreviewMode = 'shaded' | 'wireframe' | 'uv' | 'bones';

/** The file a preview shows and what it reports about it. */
export interface ModelSource {
  /** A URL the web view can fetch: the asset protocol, or a mock file. */
  url: string | null;
  onLoaded?: (info: { meshes: number; bones: number }) => void;
  onError?: (message: string) => void;
}

export interface ModelPreviewProps extends ModelSource {
  mode?: PreviewMode;
  /** Spin slowly until someone drags. */
  turntable?: boolean;
  className?: string;
}

/** A loaded model: its scene, the materials it came with and its bones. */
export interface Loaded {
  scene: THREE.Group;
  originals: Originals;
  bones: THREE.Bone[];
}

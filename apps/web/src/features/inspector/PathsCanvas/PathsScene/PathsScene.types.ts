import type { RefObject } from 'react';
import type * as THREE from 'three';
import type { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import type { PathsCanvasProps } from '../PathsCanvas.types';

export type PathsSceneProps = Omit<PathsCanvasProps, 'className'>;

/** The scene's three.js objects and the refs its hooks share. */
export interface PathsRig {
  gl: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  /** Each model's holder by asset id, once they loaded. */
  holders: Map<string, THREE.Group> | null;
  /** The latest props, for listeners that outlive a render. */
  live: RefObject<PathsSceneProps>;
  orbit: OrbitControls;
  transform: TransformControls;
  /** The models. */
  world: THREE.Group;
  /** A tube per path, `userData.path` its id. */
  tubes: THREE.Group;
  /** The selected path's handles, `userData.point` their index. */
  handles: THREE.Group;
  raycaster: THREE.Raycaster;
}

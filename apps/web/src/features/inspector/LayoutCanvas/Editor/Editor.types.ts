import type { RefObject } from 'react';
import type * as THREE from 'three';
import type { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { TransformControls } from 'three/examples/jsm/controls/TransformControls.js';
import type { CameraPose } from '@/state/inspector';
import type { LayoutCanvasProps } from '../LayoutCanvas.types';

export type EditorProps = Omit<LayoutCanvasProps, 'className'>;

/** A camera flight in progress. */
export interface Flight {
  from: CameraPose;
  to: CameraPose;
  start: number;
}

/** The editor's three.js objects and the refs its hooks share. */
export interface EditorRig {
  gl: THREE.WebGLRenderer;
  scene: THREE.Scene;
  size: { width: number; height: number };
  camera: THREE.PerspectiveCamera;
  /** Each model's holder by asset id, once they loaded. */
  holders: Map<string, THREE.Group> | null;
  /** The latest props, for listeners that outlive a render. */
  live: RefObject<EditorProps>;
  orbit: OrbitControls;
  transform: TransformControls;
  world: THREE.Group;
  stopsGroup: THREE.Group;
  box: THREE.Box3Helper;
  raycaster: THREE.Raycaster;
  flightRef: RefObject<Flight | null>;
  /** The scale when a gizmo drag started. */
  dragStart: RefObject<number>;
  /** A widened preview field of view and the stop's own, so poses report the stop's. */
  fitted: RefObject<{ shown: number; real: number } | null>;
  /** Tells the editor where the camera stands. */
  report: () => void;
}

import type { Dispatch, RefObject, SetStateAction } from 'react';
import type * as THREE from 'three';
import type { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import type { Originals } from '@/features/viewer/materials';
import type { Framing } from '@/features/viewer/studio';
import type { CameraPose, Stop } from '@/state/inspector';
import type { CompareCanvasProps } from '../CompareCanvas.types';

export type CompareProps = Omit<CompareCanvasProps, 'className'>;

/** One side's models in one group, with their materials and animation mixers. */
export interface Side {
  group: THREE.Group;
  originals: Originals;
  mixers: THREE.AnimationMixer[];
}

/** Frame time bookkeeping for the current sample window. */
export interface FrameStats {
  windowStart: number;
  frames: number;
  timed: number;
  work: number;
  last: number;
}

/** The compare view's three.js objects and the refs its hooks share. */
export interface CompareRig {
  gl: THREE.WebGLRenderer;
  size: { width: number; height: number };
  camera: THREE.PerspectiveCamera;
  left: Side | null;
  right: Side | null;
  scenes: { left: THREE.Scene; right: THREE.Scene };
  controls: OrbitControls;
  checker: THREE.Texture;
  framing: Framing | null;
  setFraming: Dispatch<SetStateAction<Framing | null>>;
  /** The latest props, for listeners and the frame loop. */
  live: RefObject<CompareProps>;
  flightRef: RefObject<{ from: CameraPose; stop: Stop; start: number } | null>;
  /** Stops without a poster still wait for their live render. */
  shotsPending: RefObject<boolean>;
  stats: RefObject<FrameStats>;
  /** The one pixel a timed frame reads back. */
  pixel: Uint8Array;
  raycaster: THREE.Raycaster;
}

/**
 * The tour's camera: the hero frame, then the six stops of the design's
 * camera board, each with its framing and the move it plays while the page
 * scrolls across it (see ./path.ts). Same coordinates as the manifest.
 */
import type { StopId, Vec3 } from './manifest';

/**
 * What the camera does across a stop, as the total change over the move.
 * The stop's framing sits in the middle of the move (at the end of it for
 * the last stop), so a poster of the stop is the frame the scroll shows.
 */
export interface CameraMove {
  /** Metres toward the target; negative pulls back. */
  dolly?: number;
  /** Metres the camera rises while the target stays, tilting the view down. */
  rise?: number;
  /** Metres the camera and the target slide together. */
  truck?: Vec3;
  /** Degrees the camera and the target swing about `pivot`'s vertical axis. */
  orbit?: number;
  pivot?: Vec3;
}

export interface CameraStop {
  id: StopId;
  position: Vec3;
  target: Vec3;
  /** Vertical field of view in degrees, framed for a 16:10 viewport. */
  fov: number;
  move?: CameraMove;
}

/** The clay island over its plinth, at its eye height. */
const ISLAND: Vec3 = [-0.02, 1.1, -0.04];

export const STOPS: readonly CameraStop[] = [
  { id: 'hero', position: [-0.45, 1.35, 1.3], target: [-0.3, 1.16, -0.05], fov: 36 },
  // Close on the paper sketch: a low dolly in that tilts a little further down.
  {
    id: 'concept',
    position: [-0.1, 1.44, 0.8],
    target: [-0.82, 0.84, 0.06],
    fov: 30,
    move: { dolly: 0.14, rise: 0.04 },
  },
  // The pinned prints: a truck left along the paper.
  {
    id: 'refs',
    position: [-0.42, 1.48, 0.66],
    target: [-0.96, 0.86, -0.04],
    fov: 35,
    move: { truck: [-0.16, 0, -0.03] },
  },
  // The clay maquette: about 20 degrees of orbit around the plinth at its eye height.
  {
    id: 'model',
    position: [0.32, 1.17, 0.68],
    target: [-0.2, 1.1, -0.04],
    fov: 35,
    move: { orbit: -20, pivot: ISLAND },
  },
  // Holds on the clay island while the wireframe sweep crosses it.
  { id: 'blender', position: [-0.3, 1.2, 0.62], target: [-0.2, 1.1, -0.04], fov: 40 },
  // The finished island: a push in while the turntable turns.
  {
    id: 'web',
    position: [0.62, 1.25, 0.66],
    target: [0.18, 1.16, -0.08],
    fov: 35,
    move: { dolly: 0.12 },
  },
  // Pulls back and up to the whole bench, ending on this framing.
  {
    id: 'ship',
    position: [-0.42, 1.5, 1.2],
    target: [-0.24, 0.98, -0.1],
    fov: 45,
    move: { dolly: -0.3, rise: 0.12 },
  },
];

/**
 * Framings used only to render posters (see scripts/capture-posters.mjs).
 * The phone poster is portrait and sits close on the two islands.
 */
export const POSTER_CAMERAS = {
  mobile: { id: 'hero', position: [0.1, 1.3, 0.78], target: [0.2, 1.04, -0.06], fov: 40 },
} as const satisfies Record<string, CameraStop>;

export type PosterCamera = keyof typeof POSTER_CAMERAS;

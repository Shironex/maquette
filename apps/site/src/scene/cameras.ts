/**
 * The tour's camera: the hero frame, then the six stops of the design's
 * camera board. Each stop has its framing, the move it plays while its copy
 * is on screen and the poses the camera flies through on its way in from
 * the stop before (see ./path.ts). Same coordinates as the manifest.
 */
import type { Vec3 } from './manifest';
import { STOP_IDS, type StopId } from './stops';

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

/** A pose on the flight into a stop, `at` stops before it (a negative offset). */
export interface Via {
  at: number;
  position: Vec3;
  target: Vec3;
}

export interface CameraStop {
  id: StopId;
  position: Vec3;
  target: Vec3;
  /** Vertical field of view in degrees, framed for a 16:10 viewport. */
  fov: number;
  move?: CameraMove;
  via?: readonly Via[];
  /** The side of the screen the stop's copy takes, the one its subject leaves clear (left by default). */
  copy?: 'left' | 'right';
}

/** What the site adds to a stop's framing: its move, the flight in and the copy's side. */
export type StopMotion = Pick<CameraStop, 'move' | 'via' | 'copy'>;

/** The clay island over its plinth, at its eye height. */
const CLAY: Vec3 = [-0.02, 1.14, -0.04];

/**
 * The framings come from the shipped layout (see ./manifest.ts); the site
 * keeps how the camera moves through them, tuned to those framings.
 */
export const STOP_MOTION: Readonly<Record<StopId, StopMotion>> = {
  hero: { move: { dolly: 0.1 }, copy: 'left' },
  // Swoops down to the right, low over the tools and past the plinth, then
  // sweeps back left onto the paper sketch: a low dolly in that tilts
  // further down.
  concept: {
    move: { dolly: 0.18, rise: 0.05 },
    via: [{ at: -0.55, position: [0.3, 1.07, 0.7], target: [-0.16, 0.93, 0.2] }],
    copy: 'left',
  },
  // Crosses back over the paper, low, then trucks left along it to the
  // pinned prints with the book stack behind them.
  refs: {
    move: { truck: [-0.22, 0, -0.02] },
    via: [{ at: -0.5, position: [-0.22, 1.05, 0.52], target: [-0.5, 0.91, 0.02] }],
    copy: 'right',
  },
  // Rises off the paper and swings across the front of the plinth, the clay
  // island passing close, then orbits it at its eye height.
  model: {
    move: { orbit: 20, pivot: CLAY },
    via: [
      { at: -0.6, position: [-0.42, 1.3, 0.72], target: [-0.3, 1.0, -0.04] },
      { at: -0.35, position: [-0.06, 1.08, 0.42], target: [-0.04, 1.12, -0.04] },
    ],
    copy: 'left',
  },
  // Cranes up and back over the plinth and holds on the clay island while
  // the wireframe sweep crosses it.
  blender: {
    move: { dolly: 0.06 },
    via: [{ at: -0.5, position: [0.24, 1.36, 0.58], target: [-0.02, 1.12, -0.04] }],
    copy: 'right',
  },
  // Drops low past the calipers and turns onto the finished island: a push
  // in while the turntable turns.
  web: {
    move: { dolly: 0.2 },
    via: [{ at: -0.5, position: [0.42, 1.02, 0.5], target: [0.3, 1.1, -0.06] }],
    copy: 'left',
  },
  // Pulls back and up to the whole bench, ending on this framing.
  ship: {
    move: { dolly: -0.3, rise: 0.12 },
    via: [{ at: -0.65, position: [0.5, 1.4, 0.95], target: [0.2, 1.08, -0.08] }],
    copy: 'left',
  },
};

/** A whole bench view, for a layout that ships without stops. */
const OVERVIEW: Omit<CameraStop, 'id'> = {
  position: [-0.42, 1.5, 1.2],
  target: [-0.24, 0.98, -0.1],
  fov: 45,
};

/**
 * One stop per site stop, in order: each framing from `framings` (a stop
 * the layout lacks keeps the last framing it has, or the bench overview)
 * with the site's motion.
 */
export function withMotion(framings: readonly CameraStop[]): CameraStop[] {
  return STOP_IDS.map((id, index) => {
    const { position, target, fov } = framings[index] ?? framings.at(-1) ?? OVERVIEW;
    return { id, position, target, fov, ...STOP_MOTION[id] };
  });
}

/**
 * Framings used only to render posters (see scripts/capture-posters.mjs).
 * The phone poster is portrait and sits close on the two islands.
 */
export const POSTER_CAMERAS = {
  mobile: {
    id: 'hero',
    position: [0.1, 1.3, 0.78],
    target: [0.2, 1.04, -0.06],
    fov: 40,
  },
} as const satisfies Record<string, CameraStop>;

export type PosterCamera = keyof typeof POSTER_CAMERAS;

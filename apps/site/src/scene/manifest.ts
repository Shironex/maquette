/**
 * The scene manifest: every model on the workbench, where it stands, which
 * camera stops see it and what the site changes in its materials.
 *
 * Coordinates are glTF: metres, Y up, the camera side is +Z. The floor is
 * y = 0 and the bench top is at y = 0.90 (see BENCH_TOP).
 *
 * The props and the camera framings come from the Maquette project: its Ship
 * step writes ./shipped.json, and `fromLayout` (./layout.ts) turns its
 * layout into model entries and stops. The site adds what the layout does
 * not know: the set pieces built for the bench, each prop's extras
 * (./props.ts) and the camera's motion between stops (./cameras.ts).
 */

import { withBase } from '../lib/with-base';
import { withMotion, type CameraStop } from './cameras';
import { fromLayout, type SiteExtras } from './layout';
import { PROP_EXTRAS } from './props';
import rawShipped from './shipped.json';
import { parseShipped } from './shipped';
import { STOP_IDS, type StopId } from './stops';
import type { ScenePath } from './paths';
import type { PathTrack } from './timeline';
import type { TimelineTracks } from './tracks';

export { STOP_IDS, type StopId };
export { POSTER_CAMERAS, type CameraMove, type CameraStop, type PosterCamera } from './cameras';

export type Vec3 = readonly [number, number, number];

/** Height of the bench top above the floor, in metres. */
export const BENCH_TOP = 0.9;

/** Recolours one band of a single mesh model, measured up from its bottom. */
export interface LowerBand {
  /** Top of the band as a share of the model's height, 0 to 1. */
  below: number;
  color: string;
  roughness: number;
  metalness: number;
}

/**
 * What the site still changes in a model's materials at load. Colour,
 * roughness, metal and glow tweaks are baked into the files by Maquette's
 * Compress step; these are the scene's own needs.
 */
export interface MaterialOverride {
  /**
   * The pipeline bakes light into an emissive map on the second UV set. On
   * this bench the scene lights do the lighting, so by default that map
   * becomes ambient occlusion; 'off' drops it.
   */
  bakedLight?: 'ao' | 'off';
  /** Emissive strength, for the window sky. */
  emissiveIntensity?: number;
  lower?: LowerBand;
}

export interface ScreenDecal {
  /** Public path of the texture. */
  texture: string;
  /** Size of the visible screen in metres. */
  size: readonly [number, number];
  /** Relative to the model's anchor, after its rotation and scale. */
  position: Vec3;
  /** Degrees, XYZ. */
  rotation: Vec3;
}

export interface ModelEntry {
  /** Unique; the same file may appear more than once. */
  id: string;
  /** Under public/scene/models/. */
  file: string;
  position: Vec3;
  /** Degrees, applied X, then Y, then Z (three.js' default order). */
  rotation?: Vec3;
  /** Uniform. */
  scale?: number;
  /**
   * The point the file already stands at in its own coordinates (a model
   * Maquette exported in place): it is moved back by it, so `position` is
   * the point it turns about, and `rotation` and `scale` stay unset.
   */
  inPlace?: Vec3;
  /**
   * 'origin' puts the model's own origin at `position` (layout.json's rule).
   * 'bottom-center' puts the bottom centre of its bounds there instead, for
   * models whose files still carry an offset from an earlier scene.
   */
  anchor: 'origin' | 'bottom-center';
  /** The stops that see it. The current stop's models load first. */
  stops: readonly StopId[];
  material?: MaterialOverride;
  castShadow?: boolean;
  receiveShadow?: boolean;
  /** Turns about its anchor while this stop is shown (motion allowed). */
  spinAt?: StopId;
  /** Gets the wireframe sweep while this stop is shown. */
  sweepAt?: StopId;
  screen?: ScreenDecal;
  /** Why this entry is not final, shown in the README's placeholder list. */
  placeholder?: string;
}

const ALL: readonly StopId[] = STOP_IDS;
const TOP = BENCH_TOP;

/** The clay island hangs over the plinth. */
const PLINTH: Vec3 = [-0.02, TOP, -0.04];
const PLINTH_HEIGHT = 0.2658 * 0.3;

/** Set pieces built in Blender for the bench at real size, origin at the floor. */
const SET_PIECES: readonly ModelEntry[] = [
  {
    id: 'workbench',
    file: 'workbench_top.glb',
    position: [0, 0, 0],
    anchor: 'origin',
    stops: ALL,
    receiveShadow: true,
    castShadow: true,
  },
  {
    id: 'back-wall',
    file: 'back_wall_window.glb',
    position: [0, 0, -0.795],
    anchor: 'origin',
    stops: ['hero', 'model', 'web', 'ship'],
    receiveShadow: true,
    material: { emissiveIntensity: 2.4 },
  },
  {
    id: 'tracing-sheets',
    file: 'tracing_sheets.glb',
    position: [-0.6, TOP + 0.0004, -0.02],
    rotation: [0, 10, 0],
    anchor: 'origin',
    stops: ['hero', 'concept', 'refs', 'ship'],
    receiveShadow: true,
  },
  {
    id: 'sketch-paper',
    file: 'sketch_paper.glb',
    position: [-0.4, TOP + 0.0012, 0.12],
    rotation: [0, -4, 0],
    anchor: 'origin',
    stops: ['hero', 'concept', 'refs', 'ship'],
    receiveShadow: true,
  },
];

/** Clay crumbs, one file reused at three sizes, drawn after the props. */
const CRUMBS: readonly ModelEntry[] = [
  {
    id: 'crumbs-front',
    file: 'clay_crumbs.glb',
    position: [0.04, TOP, 0.13],
    anchor: 'origin',
    stops: ['hero', 'model', 'blender', 'ship'],
    receiveShadow: true,
  },
  {
    id: 'crumbs-plinth',
    file: 'clay_crumbs.glb',
    position: [PLINTH[0], TOP + PLINTH_HEIGHT - 0.003, PLINTH[2]],
    rotation: [0, 90, 0],
    scale: 0.55,
    anchor: 'origin',
    stops: ['hero', 'model', 'blender'],
  },
  {
    id: 'crumbs-bowl',
    file: 'clay_crumbs.glb',
    position: [0.6, TOP, 0.05],
    rotation: [0, 200, 0],
    scale: 0.7,
    anchor: 'origin',
    stops: ['hero', 'web', 'ship'],
  },
];

/** What Maquette shipped: the props, their files and the layout. */
export const SHIPPED = parseShipped(rawShipped);

/** Each shipped model with its file and the site's extras for it. */
function shippedAssets(): Record<string, SiteExtras> {
  return Object.fromEntries(
    Object.entries(SHIPPED.models).map(([asset, model]) => [
      asset,
      {
        anchor: 'origin',
        castShadow: true,
        receiveShadow: true,
        ...PROP_EXTRAS[asset],
        file: model.file,
        exportedInPlace: model.inPlace,
      } satisfies SiteExtras,
    ])
  );
}

const shipped = fromLayout(SHIPPED.layout, { assets: shippedAssets(), lift: BENCH_TOP });

export const MODELS: readonly ModelEntry[] = [...SET_PIECES, ...shipped.models, ...CRUMBS];

/** The hero frame, then Concept to Ship: the shipped framings with the site's motion. */
export const STOPS: readonly CameraStop[] = withMotion(shipped.stops);

/** The scroll timeline Maquette shipped: the stops it counts in, each prop's states and explode tracks. */
export const TIMELINE: TimelineTracks = {
  stopIds: SHIPPED.layout.stops.slice(0, STOP_IDS.length).map(stop => stop.id),
  states: SHIPPED.layout.states ?? {},
  explode: SHIPPED.layout.timeline?.explode ?? {},
};

/** The paths Maquette shipped, lifted onto the bench, in the order they draw. */
export const PATHS: readonly ScenePath[] = shipped.paths;

/** How much of each path is lit at each keyed stop, by path id. */
export const PATH_TRACKS: Record<string, PathTrack> = SHIPPED.layout.timeline?.paths ?? {};

/** The look the 3D layer fades into and how much of the hero copy shows, by keyed stop. */
export const STYLE_TRACK = SHIPPED.layout.timeline?.style ?? null;
export const TEXT_TRACK = SHIPPED.layout.timeline?.text ?? null;

export function modelUrl(file: string): string {
  return withBase(`/scene/models/${file}`);
}

/**
 * The order models load in for a page opened at `current`: first the ones
 * that stop sees, then the ones the nearest stops see, each group in
 * manifest order. A file listed twice loads once.
 */
export function loadOrder(models: readonly ModelEntry[], current: StopId): string[] {
  const at = STOP_IDS.indexOf(current);
  const distance = (entry: ModelEntry) =>
    Math.min(...entry.stops.map(stop => Math.abs(STOP_IDS.indexOf(stop) - at)));
  const ranked = models
    .map((entry, index) => ({ entry, index, distance: distance(entry) }))
    .sort((a, b) => a.distance - b.distance || a.index - b.index);
  return [...new Set(ranked.map(({ entry }) => entry.file))];
}

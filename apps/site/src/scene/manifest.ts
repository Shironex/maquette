/**
 * The scene manifest: every model on the workbench, where it stands, which
 * camera stops see it and how its materials are adjusted, plus the six
 * camera stops of the tour and the hero frame before them.
 *
 * Coordinates are glTF: metres, Y up, the camera side is +Z. The floor is
 * y = 0 and the bench top is at y = 0.90 (see BENCH_TOP).
 *
 * Hand placed for now, to match the approved hero concept. Later it is
 * generated from the Maquette project (see `fromLayout` in ./layout.ts and
 * the README): each `layout.json` placement becomes a model entry with
 * `anchor: 'origin'`, and each layout stop becomes a camera stop. The site
 * only adds what the layout does not know: load priority (from `stops`),
 * material overrides and the laptop screen.
 */

import { STOP_IDS, type StopId } from './stops';

export { STOP_IDS, type StopId };

export type Vec3 = readonly [number, number, number];

/** Height of the bench top above the floor, in metres. */
export const BENCH_TOP = 0.9;

export interface CameraStop {
  id: StopId;
  position: Vec3;
  target: Vec3;
  /** Vertical field of view in degrees, framed for a 16:10 viewport. */
  fov: number;
}

/** Recolours one band of a single mesh model, measured up from its bottom. */
export interface LowerBand {
  /** Top of the band as a share of the model's height, 0 to 1. */
  below: number;
  color: string;
  roughness: number;
  metalness: number;
}

export interface MaterialOverride {
  /** Replaces the base colour factor; the colour map stays and is multiplied by it. */
  color?: string;
  /** Mixes the base colour factor toward this colour by `tintAmount` (default 1). */
  tint?: string;
  tintAmount?: number;
  roughness?: number;
  metalness?: number;
  /** Ignores the model's roughness and metal map so the factors above apply evenly. */
  flatSurface?: boolean;
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

/** Where the finished island turns. */
const TURNTABLE: Vec3 = [0.44, TOP, -0.08];
/** The clay island hangs over the plinth. */
const PLINTH: Vec3 = [-0.02, TOP, -0.04];
const PLINTH_HEIGHT = 0.2658 * 0.3;

export const MODELS: readonly ModelEntry[] = [
  // Set pieces, built in Blender at real size, origin at the floor.
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

  // The two heroes and what they stand on.
  {
    id: 'plaster-plinth',
    file: 'plaster_plinth_v1.glb',
    position: PLINTH,
    rotation: [0, 4, 0],
    scale: 0.3,
    anchor: 'bottom-center',
    stops: ['hero', 'model', 'blender', 'ship'],
    castShadow: true,
    receiveShadow: true,
  },
  {
    id: 'clay-island',
    file: 'clay_island_maquette_v1.glb',
    position: [PLINTH[0], TOP + PLINTH_HEIGHT - 0.004, PLINTH[2]],
    rotation: [0, 200, 0],
    scale: 0.33,
    anchor: 'bottom-center',
    stops: ['hero', 'model', 'blender', 'ship'],
    castShadow: true,
    receiveShadow: true,
    sweepAt: 'blender',
  },
  {
    id: 'turntable',
    file: 'turntable_v1.glb',
    position: TURNTABLE,
    scale: 0.42,
    anchor: 'bottom-center',
    stops: ['hero', 'web', 'ship'],
    castShadow: true,
    receiveShadow: true,
    spinAt: 'web',
    // The top glared white and the base read as wood: a matte top and an iron base.
    material: {
      roughness: 0.82,
      metalness: 0,
      flatSurface: true,
      lower: { below: 0.62, color: '#26231f', roughness: 0.55, metalness: 0.75 },
    },
  },
  {
    id: 'finished-island',
    file: 'finished_island_v1.glb',
    position: [TURNTABLE[0], TOP + 0.126 + 0.004, TURNTABLE[2]],
    rotation: [0, 200, 0],
    scale: 0.32,
    anchor: 'bottom-center',
    stops: ['hero', 'web', 'ship'],
    castShadow: true,
    receiveShadow: true,
    spinAt: 'web',
  },

  // Set dressing.
  {
    id: 'desk-lamp',
    file: 'desk_lamp_v1.glb',
    position: [0.7, TOP, -0.29],
    rotation: [0, 200, 0],
    scale: 1,
    anchor: 'bottom-center',
    stops: ['hero', 'web', 'ship'],
    castShadow: true,
  },
  {
    id: 'book-stack',
    file: 'book_stack_v1.glb',
    position: [-0.68, TOP, -0.27],
    rotation: [0, 18, 0],
    scale: 0.44,
    anchor: 'bottom-center',
    stops: ['hero', 'refs', 'ship'],
    castShadow: true,
    receiveShadow: true,
  },
  {
    id: 'brush-cup',
    file: 'brush_cup_v1.glb',
    position: [0.78, TOP, -0.06],
    scale: 1.15,
    anchor: 'bottom-center',
    stops: ['hero', 'web', 'ship'],
    castShadow: true,
    receiveShadow: true,
  },
  {
    id: 'clay-bowl',
    file: 'clay_bowl_v1.glb',
    position: [0.7, TOP, 0.17],
    scale: 0.47,
    anchor: 'bottom-center',
    stops: ['hero', 'web', 'ship'],
    castShadow: true,
    receiveShadow: true,
  },
  {
    id: 'calipers',
    file: 'calipers_v1.glb',
    position: [0.38, TOP, 0.22],
    rotation: [0, 75, 0],
    scale: 0.15,
    anchor: 'bottom-center',
    stops: ['hero', 'web', 'ship'],
    castShadow: true,
    receiveShadow: true,
    // Read as brass; calipers are dark steel.
    material: { tint: '#5d6266', tintAmount: 1, metalness: 0.85, roughness: 0.42 },
  },
  {
    id: 'loop-tool',
    file: 'loop_tool_v1.glb',
    position: [0.06, TOP, 0.25],
    rotation: [0, 62, 0],
    scale: 0.11,
    anchor: 'bottom-center',
    stops: ['hero', 'model', 'blender', 'ship'],
    castShadow: true,
    receiveShadow: true,
  },
  {
    id: 'linen-cloth',
    file: 'linen_cloth_v1.glb',
    position: [0.8, TOP, 0.37],
    rotation: [0, 30, 0],
    scale: 0.12,
    anchor: 'bottom-center',
    stops: ['hero', 'ship'],
    castShadow: true,
    receiveShadow: true,
    // Grey from Tripo; the concept's cloth is olive.
    material: { tint: '#6f7446', tintAmount: 1, roughness: 0.95 },
  },
  {
    id: 'laptop',
    file: 'laptop_v1.glb',
    position: [-0.36, TOP, -0.26],
    rotation: [0, 22, 0],
    scale: 1.2,
    anchor: 'bottom-center',
    stops: ['hero', 'ship'],
    castShadow: true,
    receiveShadow: true,
    // Rendered as polished chrome: matte graphite instead.
    material: { color: '#3a3936', metalness: 0.25, roughness: 0.62, flatSurface: true },
    screen: {
      texture: '/scene/laptop-screen.webp',
      size: [0.27, 0.169],
      position: [0, 0.135, -0.12],
      rotation: [-14, 0, 0],
    },
    placeholder: 'The screen is a mock mode capture of the Board; swap in a real project capture.',
  },

  // Clay crumbs, one file reused at three sizes.
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

export const STOPS: readonly CameraStop[] = [
  { id: 'hero', position: [-0.45, 1.35, 1.3], target: [-0.3, 1.16, -0.05], fov: 36 },
  { id: 'concept', position: [-0.1, 1.44, 0.8], target: [-0.82, 0.84, 0.06], fov: 32 },
  { id: 'refs', position: [-0.42, 1.48, 0.66], target: [-0.96, 0.86, -0.04], fov: 34 },
  { id: 'model', position: [0.32, 1.17, 0.68], target: [-0.2, 1.1, -0.04], fov: 35 },
  { id: 'blender', position: [-0.3, 1.2, 0.62], target: [-0.2, 1.1, -0.04], fov: 40 },
  { id: 'web', position: [0.62, 1.25, 0.66], target: [0.18, 1.16, -0.08], fov: 35 },
  { id: 'ship', position: [-0.42, 1.5, 1.2], target: [-0.24, 0.98, -0.1], fov: 45 },
];

/**
 * Framings used only to render posters (see scripts/capture-posters.mjs).
 * The phone poster is portrait and sits close on the two islands.
 */
export const POSTER_CAMERAS = {
  mobile: { id: 'hero', position: [0.1, 1.3, 0.78], target: [0.2, 1.04, -0.06], fov: 40 },
} as const satisfies Record<string, CameraStop>;

export type PosterCamera = keyof typeof POSTER_CAMERAS;

export function modelUrl(file: string): string {
  return `/scene/models/${file}`;
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

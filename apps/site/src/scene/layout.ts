/**
 * Turns a Maquette project's `layout.json` into the site's manifest.
 *
 * The layout keys each placement by asset id and lists the camera stops in
 * tour order (the first one is the view the page opens on). The site adds
 * what the layout does not carry: the shipped file of each asset, material
 * overrides and extras like the laptop screen.
 *
 * layout.json, as Maquette writes it (glTF coordinates, metres, degrees):
 *
 *   {
 *     "version": 1,
 *     "models": {
 *       "desk-lamp": { "position": [0.42, 0, -0.1], "rotation": [0, -35, 0], "scale": 0.4 }
 *     },
 *     "stops": [
 *       { "id": "concept", "name": "Concept", "position": [0.9, 0.5, 1.4],
 *         "target": [0, 0.15, 0], "fov": 35, "models": null }
 *     ],
 *     "desk": { "size": [1.6, 0.8] }
 *   }
 *
 * A stop's `models` lists the asset ids it sees; `null` means it sees all.
 * Maquette's desk top is at y = 0, so `lift` raises everything onto the bench.
 */
import { STOP_IDS, type CameraStop, type ModelEntry, type StopId, type Vec3 } from './manifest';

export interface LayoutPlacement {
  position: Vec3;
  rotation: Vec3;
  scale: number;
}

export interface LayoutStop {
  id: string;
  name: string;
  position: Vec3;
  target: Vec3;
  fov: number;
  models: string[] | null;
}

export interface SceneLayout {
  version: number;
  models: Record<string, LayoutPlacement>;
  stops: LayoutStop[];
  desk?: { size: readonly [number, number] } | null;
}

/** What the site keeps per asset on top of its placement. */
export type SiteExtras = Omit<ModelEntry, 'id' | 'position' | 'rotation' | 'scale' | 'stops'>;

export interface FromLayoutOptions {
  /** Asset id to its shipped file and site extras. Assets without one are skipped. */
  assets: Record<string, SiteExtras>;
  /** Metres added to every Y, model and camera alike. */
  lift?: number;
}

export const LAYOUT_VERSION = 1;

/**
 * The layout's stops map onto the site's stops in order: the first is the
 * hero frame, then Concept to Ship. Extra stops are ignored.
 */
export function fromLayout(
  layout: SceneLayout,
  { assets, lift = 0 }: FromLayoutOptions
): { models: ModelEntry[]; stops: CameraStop[] } {
  if (layout.version !== LAYOUT_VERSION) {
    throw new Error(`layout.json version ${layout.version} is not ${LAYOUT_VERSION}`);
  }
  const up = ([x, y, z]: Vec3): Vec3 => [x, y + lift, z];
  const tour = layout.stops.slice(0, STOP_IDS.length);
  const stopId = (index: number): StopId => STOP_IDS[index] ?? 'hero';

  const stops: CameraStop[] = tour.map((stop, index) => ({
    id: stopId(index),
    position: up(stop.position),
    target: up(stop.target),
    fov: stop.fov,
  }));

  const seenBy = (asset: string): StopId[] => {
    const seen = tour.flatMap((stop, index) =>
      stop.models === null || stop.models.includes(asset) ? [stopId(index)] : []
    );
    return seen.length ? seen : ['hero'];
  };

  const models: ModelEntry[] = Object.entries(layout.models).flatMap(([asset, placement]) => {
    const extras = assets[asset];
    if (!extras) return [];
    return [
      {
        ...extras,
        id: asset,
        position: up(placement.position),
        rotation: placement.rotation,
        scale: placement.scale,
        stops: seenBy(asset),
      },
    ];
  });

  return { models, stops };
}

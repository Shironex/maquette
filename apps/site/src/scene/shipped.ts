/**
 * What Maquette's Ship step writes into the site (`shipped.json`, where
 * `maquette.site.json` points it): the shipped model of each asset, the
 * project's `layout.json` and `materials.json` as they were shipped, and
 * the posters it copied, if the site asked for them.
 *
 *   {
 *     "version": 1,
 *     "project": "maquette",
 *     "models": {
 *       "desk-lamp": { "name": "Desk lamp", "file": "desk_lamp_v1.glb", "inPlace": true }
 *     },
 *     "layout": { ...layout.json },
 *     "materials": { ...materials.json },
 *     "posters": null
 *   }
 *
 * `inPlace` says the file already stands where the layout puts it, which is
 * how the pipeline exports every model. The material tweaks are baked into
 * the shipped files by the Compress step; the copy here records what each
 * file was shipped with and is never applied again.
 */
import { LAYOUT_VERSION, type SceneLayout } from './layout';

export const SHIPPED_VERSION = 1;

export interface ShippedModel {
  name: string;
  /** Under the site's models folder. */
  file: string;
  inPlace: boolean;
}

export interface ShippedPoster {
  file: string;
  label?: string;
}

export interface ShippedScene {
  version: number;
  project: string;
  /** Asset id to its shipped model, in the order the models load and draw. */
  models: Record<string, ShippedModel>;
  layout: SceneLayout;
  /** `materials.json` as shipped: asset id to its tweaks. */
  materials: { version: number; models: Record<string, unknown> };
  /** The posters' folder in the site and the stops they show, when copied. */
  posters: { folder: string; stops: ShippedPoster[] } | null;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/**
 * Checks the shape the site relies on and returns the data typed. Throws on
 * a file from a newer Maquette or one that is not shipped data at all, so a
 * bad ship fails the build instead of rendering an empty bench.
 */
export function parseShipped(raw: unknown): ShippedScene {
  if (!isRecord(raw)) throw new Error('shipped.json is not an object');
  if (raw.version !== SHIPPED_VERSION) {
    throw new Error(`shipped.json version ${String(raw.version)} is not ${SHIPPED_VERSION}`);
  }
  const { models, layout } = raw;
  if (!isRecord(models)) throw new Error('shipped.json has no models');
  for (const [asset, model] of Object.entries(models)) {
    if (!isRecord(model) || typeof model.file !== 'string' || !model.file.endsWith('.glb')) {
      throw new Error(`shipped.json: ${asset} has no .glb file`);
    }
  }
  if (!isRecord(layout) || layout.version !== LAYOUT_VERSION) {
    throw new Error(`shipped.json: the layout is not version ${LAYOUT_VERSION}`);
  }
  if (!isRecord(layout.models) || !Array.isArray(layout.stops)) {
    throw new Error('shipped.json: the layout has no models or stops');
  }
  return raw as unknown as ShippedScene;
}

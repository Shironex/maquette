/** The compression preset table: rows for every model, and what to save. */
import type { CompressPreset, MeshoptLevel, PipelineView, TextureCodec } from '@/ipc/client';

export interface PresetRow {
  model: string;
  label: string;
  textureSize: number;
  /** A smaller size for metal, roughness, occlusion and normal maps, or null for one size. */
  dataTextureSize: number | null;
  codec: TextureCodec;
  meshopt: MeshoptLevel;
  lock: string | null;
  /** True when the row is stored (or edited), not just the defaults. */
  custom: boolean;
}

/** "sakura_large" reads as "Sakura large". */
export function modelLabel(stem: string): string {
  const words = stem.replace(/[_-]+/g, ' ').trim().toLowerCase();
  return words ? words[0]!.toUpperCase() + words.slice(1) : stem;
}

/** One row per known model: its stored preset, or the workspace defaults. */
export function presetRows(view: PipelineView): PresetRow[] {
  const stored = new Map(view.def.presets.map(p => [p.model, p]));
  // An export of an asset version reads as the asset: "clay", not "Clay v1".
  const named = new Map(view.models.map(m => [m.model, m.assetName]));
  const models = new Set([...view.models.map(m => m.model), ...stored.keys()]);
  return [...models].sort().map(model => {
    const preset = stored.get(model);
    return preset
      ? { ...preset, dataTextureSize: preset.dataTextureSize ?? null, custom: true }
      : {
          model,
          label: named.get(model) ?? modelLabel(model),
          textureSize: view.defaults.textureSize,
          dataTextureSize: null,
          codec: view.defaults.codec,
          meshopt: view.defaults.meshopt,
          lock: null,
          custom: false,
        };
  });
}

/** Stored and edited rows go back; untouched default rows stay implicit. */
export function presetsToSave(rows: readonly PresetRow[]): CompressPreset[] {
  return rows
    .filter(r => r.custom)
    .map(({ model, label, textureSize, dataTextureSize, codec, meshopt, lock }) => ({
      model,
      label,
      textureSize,
      dataTextureSize,
      codec,
      meshopt,
      lock,
    }));
}

/** How much of a model's bytes the metal, roughness, occlusion and normal maps are, roughly. */
const DATA_MAP_SHARE = 0.6;

/** The size a split row really applies to its data maps: smaller than the base size, else none. */
export function splitSize(row: Pick<PresetRow, 'textureSize' | 'dataTextureSize'>): number | null {
  const data = row.dataTextureSize;
  return data !== null && data < row.textureSize ? data : null;
}

/** What a row's sizes do to the bytes, against one size for every map. */
function sizeFactor(row: Pick<PresetRow, 'textureSize' | 'dataTextureSize'>): number {
  const data = splitSize(row);
  if (data === null) return 1;
  return 1 - DATA_MAP_SHARE * (1 - (data / row.textureSize) ** 2);
}

/**
 * The bytes the models will ship after Compress with the edited rows: what
 * was built (or the raw file), scaled by how a row's split differs from the
 * one the build had. A guess, since the textures' share of a file is not known.
 */
export function estimateTransfer(view: PipelineView, rows: readonly PresetRow[]): number {
  const saved = new Map(presetRows(view).map(r => [r.model, r]));
  const edited = new Map(rows.map(r => [r.model, r]));
  return view.models.reduce((sum, m) => {
    const base = m.shippedBytes ?? m.rawBytes ?? 0;
    const was = saved.get(m.model);
    const now = edited.get(m.model);
    if (!was || !now) return sum + base;
    return sum + base * (sizeFactor(now) / sizeFactor(was));
  }, 0);
}

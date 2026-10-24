/** The compression preset table: rows for every model, and what to save. */
import type { CompressPreset, MeshoptLevel, PipelineView, TextureCodec } from '@/ipc/client';

export interface PresetRow {
  model: string;
  label: string;
  textureSize: number;
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
  const models = new Set([...view.models.map(m => m.model), ...stored.keys()]);
  return [...models].sort().map(model => {
    const preset = stored.get(model);
    return preset
      ? { ...preset, custom: true }
      : {
          model,
          label: modelLabel(model),
          textureSize: view.defaults.textureSize,
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
    .map(({ model, label, textureSize, codec, meshopt, lock }) => ({
      model,
      label,
      textureSize,
      codec,
      meshopt,
      lock,
    }));
}

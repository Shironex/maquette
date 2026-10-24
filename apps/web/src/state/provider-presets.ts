/**
 * The same preset checks the Rust `providers::presets` module runs, so a
 * bad field shows next to the control before anything is saved or sent.
 * The model ids and labels come from the catalog in the bootstrap; only
 * the rules live here.
 */
import type {
  MeshyPreset,
  ModelOption,
  PresetVariant,
  ProviderId,
  TripoPreset,
  WorkspaceConfig,
} from '@/ipc/client';
import { formatCount } from '@/lib/format';

export function isTripoPSeries(model: string): boolean {
  return /^p/i.test(model);
}

export function tripoFaceRange(preset: TripoPreset): [number, number] {
  const { model } = preset;
  if (model.startsWith('P1')) return [50, 20_000];
  if (model.startsWith('P2')) return [48, 50_000];
  if (preset.smartLowPoly) return [500, 20_000];
  if (preset.geometryQuality === 'detailed' && !model.startsWith('v2')) return [1, 2_000_000];
  if (model.startsWith('v3.1')) return [1, 1_500_000];
  if (model.startsWith('v3.0')) return [1, 1_000_000];
  return [1, 500_000];
}

export function isSmartTopology(model: string): boolean {
  return model.startsWith('meshy-t');
}

export function meshyTakesRemoveLighting(model: string): boolean {
  return !isSmartTopology(model) && model !== 'meshy-6-lite';
}

export function meshyPolycountRange(model: string): [number, number] {
  if (model === 'meshy-t2') return [100, 15_000];
  if (model === 'meshy-t2.5') return [100, 30_000];
  return [100, 300_000];
}

/** The label a catalog gives a model id, or the id itself. */
export function modelLabel(options: readonly ModelOption[], id: string): string {
  return options.find(o => o.id === id)?.label ?? id;
}

export function tripoProblem(preset: TripoPreset, models: readonly ModelOption[]): string | null {
  if (!preset.model.trim()) return 'Pick a Tripo model.';
  if (preset.faceLimit > 0) {
    const [low, high] = tripoFaceRange(preset);
    if (preset.faceLimit < low || preset.faceLimit > high) {
      return `Tripo ${modelLabel(models, preset.model)} takes a face limit from ${formatCount(
        low
      )} to ${formatCount(high)}, or 0 for adaptive.`;
    }
  }
  if (preset.rig && !preset.rigModel.trim()) return 'Pick a Tripo rig model.';
  if (preset.animations.length > 5) return 'Tripo retargets at most 5 animations at once.';
  return null;
}

export function meshyProblem(preset: MeshyPreset, models: readonly ModelOption[]): string | null {
  if (!preset.aiModel.trim()) return 'Pick a Meshy model.';
  const smart = isSmartTopology(preset.aiModel);
  const [low, high] = meshyPolycountRange(preset.aiModel);
  const target = preset.targetPolycount;
  if ((preset.shouldRemesh || smart) && (target < low || target > high)) {
    return `${modelLabel(models, preset.aiModel)} aims for ${formatCount(low)} to ${formatCount(
      high
    )} faces.`;
  }
  if (smart && preset.topology === 'quad') return 'Smart Topology models only make triangles.';
  const height = preset.heightMeters;
  if (height === null || !Number.isFinite(height) || height <= 0) {
    return 'The character height must be more than 0 m.';
  }
  return null;
}

export type PresetKey = `${ProviderId}-${PresetVariant}`;

export const VARIANTS: readonly PresetVariant[] = ['prop', 'character'];

/** Every preset problem in a config, keyed by service and variant. */
export function presetProblems(
  config: WorkspaceConfig,
  catalog: { tripoModels: readonly ModelOption[]; meshyModels: readonly ModelOption[] }
): Partial<Record<PresetKey, string>> {
  const out: Partial<Record<PresetKey, string>> = {};
  for (const variant of VARIANTS) {
    const tripo = tripoProblem(config.providers.tripo[variant], catalog.tripoModels);
    if (tripo) out[`tripo-${variant}`] = tripo;
    const meshy = meshyProblem(config.providers.meshy[variant], catalog.meshyModels);
    if (meshy) out[`meshy-${variant}`] = meshy;
  }
  return out;
}

/** One line per service for summaries: "Tripo H3.1, PBR, rig with Mixamo". */
export function tripoSummary(preset: TripoPreset, models: readonly ModelOption[]): string {
  return [
    `Tripo ${modelLabel(models, preset.model)}`,
    preset.faceLimit ? `${formatCount(preset.faceLimit)} faces` : 'adaptive faces',
    preset.pbr ? 'PBR' : null,
    preset.multiView ? 'multi-view' : null,
    preset.rig ? `rig with ${preset.rigSpec === 'mixamo' ? 'Mixamo' : 'Tripo'} bones` : null,
  ]
    .filter(Boolean)
    .join(', ');
}

export function meshySummary(preset: MeshyPreset, models: readonly ModelOption[]): string {
  const smart = isSmartTopology(preset.aiModel);
  return [
    modelLabel(models, preset.aiModel),
    smart || preset.shouldRemesh ? `${formatCount(preset.targetPolycount)} faces` : null,
    preset.enablePbr ? 'PBR' : null,
    preset.textureResolution.toUpperCase(),
    preset.multiView ? 'multi-view' : null,
    preset.rig ? 'rigged' : null,
  ]
    .filter(Boolean)
    .join(', ');
}

import type { TripoPreset } from '@/ipc/client';
import { isTripoPSeries, tripoFaceRange } from '@/state/provider-presets';
import { useCatalog } from '../useCatalog';

/** The model lists, whether the model is a P series one, and its face range. */
export function useTripoPresetFields(preset: TripoPreset) {
  const catalog = useCatalog();
  const pSeries = isTripoPSeries(preset.model);
  const [low, high] = tripoFaceRange(preset);
  return { catalog, pSeries, low, high };
}

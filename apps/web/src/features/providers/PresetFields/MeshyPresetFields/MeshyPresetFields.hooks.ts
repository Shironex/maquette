import type { MeshyPreset } from '@/ipc/client';
import { isSmartTopology, meshyPolycountRange } from '@/state/provider-presets';
import { useCatalog } from '../useCatalog';

/** The model lists, whether the model is Smart Topology, and its polycount range. */
export function useMeshyPresetFields(preset: MeshyPreset) {
  const catalog = useCatalog();
  const smart = isSmartTopology(preset.aiModel);
  const [low, high] = meshyPolycountRange(preset.aiModel);
  return { catalog, smart, low, high };
}

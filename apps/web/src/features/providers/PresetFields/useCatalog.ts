import type { ProviderCatalog } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';

const EMPTY_CATALOG: ProviderCatalog = {
  tripoModels: [],
  tripoRigModels: [],
  tripoAnimations: [],
  meshyModels: [],
};

/** The model lists from the bootstrap. */
export function useCatalog(): ProviderCatalog {
  return useAppStore(s => s.boot?.catalog) ?? EMPTY_CATALOG;
}

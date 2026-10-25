import { useProvidersStore } from '@/state/use-providers-store';

/** The batch the store has open, if any, and how to close it. */
export function useBatchPanel() {
  const batch = useProvidersStore(s => s.batch);
  const close = useProvidersStore(s => s.closeBatch);
  return { batch, close };
}

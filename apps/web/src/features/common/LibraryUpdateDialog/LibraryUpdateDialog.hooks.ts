import { useLibraryStore } from '@/state/use-library-store';

/** The update's plan once read, or why it could not be, and the calls. */
export function useLibraryUpdateDialog() {
  const plan = useLibraryStore(s => s.plan);
  const planError = useLibraryStore(s => s.planError);
  const busy = useLibraryStore(s => s.busy);
  const closeUpdate = useLibraryStore(s => s.closeUpdate);
  const update = useLibraryStore(s => s.update);
  return { plan, planError, busy, closeUpdate, update };
}

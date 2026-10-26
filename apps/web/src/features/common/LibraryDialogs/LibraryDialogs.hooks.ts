import { useLibraryStore } from '@/state/use-library-store';

/** Which library dialog is open, and for which asset. */
export function useLibraryDialogs() {
  const adding = useLibraryStore(s => s.adding);
  const updating = useLibraryStore(s => s.updating);
  return { adding, updating };
}

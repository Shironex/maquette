import { useNow } from '@/lib/useNow';
import { useTriageStore } from '@/state/use-triage-store';

/** The discarded models to list, how many more there are, and Restore. */
export function useDiscardedList(limit: number) {
  const now = useNow(30_000);
  const { discarded, restoring, restore } = useTriageStore();
  const shown = discarded.slice(0, limit);
  return {
    now,
    total: discarded.length,
    shown,
    more: discarded.length - shown.length,
    restoring,
    restore,
  };
}

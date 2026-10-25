import { useNow } from '@/lib/useNow';
import { useLabStore } from '@/state/use-lab-store';

/** Cancel all and Clear, and the clock the running jobs tick on. */
export function useImageQueue() {
  const cancelAll = useLabStore(s => s.cancelAll);
  const clearFinished = useLabStore(s => s.clearFinished);
  const now = useNow(1000);
  return { cancelAll, clearFinished, now };
}

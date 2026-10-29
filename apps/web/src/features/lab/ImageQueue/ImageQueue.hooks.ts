import { useNow } from '@/lib/useNow';
import { useLabStore } from '@/state/use-lab-store';

/** Cancel all, Clear and Run next, and the clock the running jobs tick on. */
export function useImageQueue() {
  const cancelAll = useLabStore(s => s.cancelAll);
  const clearFinished = useLabStore(s => s.clearFinished);
  const runNext = useLabStore(s => s.runNext);
  const now = useNow(1000);
  return { cancelAll, clearFinished, runNext, now };
}

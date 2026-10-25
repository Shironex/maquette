import { useNow } from '@/lib/useNow';

/** The clock the elapsed time ticks on, once a second. */
export function useJobTile() {
  return useNow(1000);
}

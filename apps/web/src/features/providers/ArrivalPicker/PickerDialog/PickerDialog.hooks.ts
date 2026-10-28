import { useRef } from 'react';
import { useNow } from '@/lib/useNow';
import { useTopEscape } from '@/lib/useTopEscape';
import { askingArrivals, lookingArrivals } from '@/state/downloads';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** The downloads waiting for a look or for their asset, and Escape to close while on top. */
export function usePickerDialog(onClose: () => void) {
  const status = useDownloadsStore(s => s.status);
  const home = useAppStore(s => s.boot?.home);
  const ref = useRef<HTMLDivElement>(null);
  const now = useNow(30_000);
  useTopEscape(ref, onClose);
  const looking = lookingArrivals(status);
  const asking = askingArrivals(status);
  return {
    looking,
    asking,
    count: looking.length + asking.length,
    folder: status?.folder ?? null,
    home,
    ref,
    now,
  };
}

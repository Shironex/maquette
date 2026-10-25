import { useRef } from 'react';
import { useNow } from '@/lib/useNow';
import { useTopEscape } from '@/lib/useTopEscape';
import { askingArrivals, sendOptions } from '@/state/downloads';
import { useAppStore } from '@/state/use-app-store';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** The downloads asking, the sends they can go to, and Escape to close while on top. */
export function usePickerDialog(onClose: () => void) {
  const status = useDownloadsStore(s => s.status);
  const projects = useAppStore(s => s.workspace?.projects);
  const home = useAppStore(s => s.boot?.home);
  const ref = useRef<HTMLDivElement>(null);
  const now = useNow(30_000);
  useTopEscape(ref, onClose);
  const nameOf = (id: string) => projects?.find(p => p.id === id)?.name ?? id;
  return {
    asking: askingArrivals(status),
    options: sendOptions(status?.pending ?? [], nameOf),
    folder: status?.folder ?? null,
    home,
    ref,
    now,
  };
}

import { useRef } from 'react';
import { useNow } from '@/lib/useNow';
import { useTopEscape } from '@/lib/useTopEscape';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** The scan to show, the clock for its ages, and Escape to close while on top. */
export function useDownloadsDialog(onClose: () => void) {
  const downloads = useProvidersStore(s => s.downloads);
  const retry = useProvidersStore(s => s.checkDownloads);
  const home = useAppStore(s => s.boot?.home);
  const ref = useRef<HTMLDivElement>(null);
  const now = useNow(30_000);
  useTopEscape(ref, onClose);
  return { downloads, retry, home, ref, now };
}

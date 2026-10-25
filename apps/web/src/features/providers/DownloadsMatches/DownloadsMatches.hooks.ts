import { useProvidersStore } from '@/state/use-providers-store';

/** Whether a Downloads check is open, and how to close it. */
export function useDownloadsOpen() {
  const open = useProvidersStore(s => Boolean(s.downloads));
  const close = useProvidersStore(s => s.closeDownloads);
  return { open, close };
}

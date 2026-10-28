import { useDownloadsStore } from '@/state/use-downloads-store';

/** Opening the asset chooser for one download, or leaving it alone. */
export function useArrivalRow(id: string) {
  const busy = useDownloadsStore(s => s.busy.includes(id));
  const openChooser = useDownloadsStore(s => s.openChooser);
  const dismiss = useDownloadsStore(s => s.dismiss);
  return { busy, choose: () => openChooser(id), dismiss: () => void dismiss(id) };
}

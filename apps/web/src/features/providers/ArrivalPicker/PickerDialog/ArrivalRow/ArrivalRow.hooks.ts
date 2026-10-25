import { useDownloadsStore } from '@/state/use-downloads-store';

/** The send picked for one download, and taking it or leaving it. */
export function useArrivalRow(id: string) {
  const choice = useDownloadsStore(s => s.choices[id] ?? null);
  const busy = useDownloadsStore(s => s.busy.includes(id));
  const choose = useDownloadsStore(s => s.choose);
  const assign = useDownloadsStore(s => s.assign);
  const dismiss = useDownloadsStore(s => s.dismiss);
  return { choice, busy, choose, assign, dismiss };
}

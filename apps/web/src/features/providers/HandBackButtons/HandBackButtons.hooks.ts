import { useProvidersStore } from '@/state/use-providers-store';

/** Bringing a by hand model back: pick a file, or look in Downloads. */
export function useHandBack(jobId: string) {
  const addModel = useProvidersStore(s => s.addModel);
  const checkDownloads = useProvidersStore(s => s.checkDownloads);
  const busy = useProvidersStore(s => s.busy.includes(jobId));
  return { addModel, checkDownloads, busy };
}

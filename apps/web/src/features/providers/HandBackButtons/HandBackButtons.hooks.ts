import { jobKey, type JobRef } from '@/state/providers';
import { useProvidersStore } from '@/state/use-providers-store';

/** Bringing a by hand model back: pick a file, or look in Downloads. */
export function useHandBack(job: JobRef) {
  const key = jobKey(job);
  const addModel = useProvidersStore(s => s.addModel);
  const checkDownloads = useProvidersStore(s => s.checkDownloads);
  const busy = useProvidersStore(s => s.busy.includes(key));
  return { addModel, checkDownloads, busy };
}

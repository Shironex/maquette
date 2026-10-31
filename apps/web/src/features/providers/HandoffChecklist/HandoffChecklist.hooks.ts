import { useNow } from '@/lib/useNow';
import { checklist, waitingByHand } from '@/state/batch';
import { useProvidersStore } from '@/state/use-providers-store';

/** The by hand sends the checklist lists and its actions. */
export function useHandoffChecklist() {
  const jobs = useProvidersStore(s => s.jobs);
  const openSite = useProvidersStore(s => s.openSite);
  const checkDownloads = useProvidersStore(s => s.checkDownloads);
  const now = useNow(30_000);
  const groups = checklist(jobs);
  const waiting = waitingByHand(jobs);
  const services = [...new Set(waiting.map(j => j.service))];
  return { groups, waiting, services, openSite, checkDownloads, now };
}

import { useNow } from '@/lib/useNow';
import { checklist, waitingByHand } from '@/state/batch';
import { useProvidersStore } from '@/state/use-providers-store';

/** Whether the checklist is open, the by hand sends it lists and its actions. */
export function useHandoffChecklist() {
  const open = useProvidersStore(s => s.checklistOpen);
  const setOpen = useProvidersStore(s => s.setChecklistOpen);
  const jobs = useProvidersStore(s => s.jobs);
  const openSite = useProvidersStore(s => s.openSite);
  const checkDownloads = useProvidersStore(s => s.checkDownloads);
  const now = useNow(30_000);
  const groups = checklist(jobs);
  const waiting = waitingByHand(jobs);
  const services = [...new Set(waiting.map(j => j.service))];
  const close = () => setOpen(false);
  return { open, groups, waiting, services, openSite, checkDownloads, close, now };
}

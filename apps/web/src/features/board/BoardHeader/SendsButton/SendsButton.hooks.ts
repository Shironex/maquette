import { sendCounts, sendsOpensOn } from '@/state/sends';
import { useProvidersStore } from '@/state/use-providers-store';

/** The counts of the open project's sends, and the list a press opens. */
export function useSendsButton() {
  const jobs = useProvidersStore(s => s.jobs);
  const setQueueOpen = useProvidersStore(s => s.setQueueOpen);
  const setChecklistOpen = useProvidersStore(s => s.setChecklistOpen);
  const counts = sendCounts(jobs);
  return {
    counts,
    open: () => (sendsOpensOn(counts) === 'hand' ? setChecklistOpen(true) : setQueueOpen(true)),
  };
}

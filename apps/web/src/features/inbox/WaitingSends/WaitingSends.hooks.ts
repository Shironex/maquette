import { useNow } from '@/lib/useNow';
import { useProvidersStore } from '@/state/use-providers-store';

/** The By hand sends still waiting for a model, with Open handoff and Stop waiting. */
export function useWaitingSends() {
  const now = useNow(30_000);
  const jobs = useProvidersStore(s => s.jobs);
  const openHandoff = useProvidersStore(s => s.openHandoff);
  const cancel = useProvidersStore(s => s.cancel);
  return {
    now,
    waiting: jobs.filter(j => j.provider === 'by-hand' && j.state === 'waiting'),
    openHandoff,
    cancel,
  };
}

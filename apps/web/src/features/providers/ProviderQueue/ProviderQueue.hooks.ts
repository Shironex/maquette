import { useNow } from '@/lib/useNow';
import { isActive, sortJobs } from '@/state/providers';
import { useProvidersStore } from '@/state/use-providers-store';

/** The sends, newest first, ticking once a second for their times. */
export function useQueueJobs() {
  const jobs = useProvidersStore(s => s.jobs);
  const clear = useProvidersStore(s => s.clearFinished);
  const project = useProvidersStore(s => s.project);
  const now = useNow(1_000);
  const sorted = sortJobs(jobs);
  const finished = jobs.some(j => !isActive(j.state) && j.state !== 'inbox');
  return { sorted, finished, clear, now, project };
}

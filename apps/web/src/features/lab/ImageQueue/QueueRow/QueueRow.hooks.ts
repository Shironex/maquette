import { useLabStore } from '@/state/use-lab-store';

export function useQueueRow() {
  const cancel = useLabStore(s => s.cancel);
  const retry = useLabStore(s => s.retry);
  const runNext = useLabStore(s => s.runNext);
  const openLightbox = useLabStore(s => s.openLightbox);
  return { cancel, retry, runNext, openLightbox };
}

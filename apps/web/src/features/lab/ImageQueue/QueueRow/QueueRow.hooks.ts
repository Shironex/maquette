import { useLabStore } from '@/state/use-lab-store';

export function useQueueRow() {
  const cancel = useLabStore(s => s.cancel);
  const openLightbox = useLabStore(s => s.openLightbox);
  return { cancel, openLightbox };
}

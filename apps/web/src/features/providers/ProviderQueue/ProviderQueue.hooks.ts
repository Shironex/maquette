import { useEffect, useRef } from 'react';
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

/** Escape or a press outside the popover (and its toggle) closes it. */
export function useDismissOutside(onClose: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    const onDown = (e: PointerEvent) => {
      const target = e.target as HTMLElement;
      if (ref.current?.contains(target) || target.closest('[data-queue-toggle]')) return;
      onClose();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('pointerdown', onDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('pointerdown', onDown);
    };
  }, [onClose]);
  return ref;
}

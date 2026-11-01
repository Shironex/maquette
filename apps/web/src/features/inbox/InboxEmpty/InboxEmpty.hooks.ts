import { useProvidersStore } from '@/state/use-providers-store';

/** The shell mounts the By hand checklist; the empty inbox only asks for it. */
export function useInboxEmpty() {
  const setChecklistOpen = useProvidersStore(s => s.setChecklistOpen);
  return { openChecklist: () => setChecklistOpen(true) };
}

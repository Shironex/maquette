import { useLabStore } from '@/state/use-lab-store';

/** The draft the dialog is open on, or null while it is closed. */
export function useObjectRefsDialog() {
  return useLabStore(s => s.objects);
}

import { useProjectsStore } from '@/state/use-projects-store';

/** The mode the dialog is open on, or null while it is closed. */
export function useNewProjectDialog() {
  return useProjectsStore(s => s.dialog);
}

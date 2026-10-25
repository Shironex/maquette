import { useAppStore } from '@/state/use-app-store';
import { useLabStore } from '@/state/use-lab-store';

/** The open project and the lab view loaded last. */
export function useLabView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const view = useLabStore(s => s.view);
  return { project, view };
}

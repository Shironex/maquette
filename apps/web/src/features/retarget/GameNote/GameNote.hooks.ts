import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';

/** The open project and its motion, which the budget note reads. */
export function useGameNote() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const view = useMotionStore(s => s.view);
  const draft = useMotionStore(s => s.draft);
  return { project, view, draft };
}

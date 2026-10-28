import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRetargetStore } from '@/state/use-retarget-store';

/** The selected model, and the ways a first clip can reach it. */
export function useNoClipsOptions() {
  const view = useMotionStore(s => s.view);
  const asset = useMotionStore(s => s.asset);
  const openRetarget = useRetargetStore(s => s.open);
  const projectId = useAppStore(s => s.workspace?.project?.id ?? null);
  const setView = useAppStore(s => s.setView);
  const setAdding = useBuiltStore(s => s.setAdding);
  const model = view?.models.find(m => m.asset === asset) ?? null;
  return {
    model,
    retarget: () => {
      if (model && projectId) void openRetarget({ project: projectId, asset: model.asset });
    },
    openInbox: () => setView('inbox'),
    addScript: () => {
      setView('board');
      setAdding(true);
    },
  };
}

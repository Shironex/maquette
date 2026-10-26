import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useMotionStore } from '@/state/use-motion-store';

/** The selected model, and the ways a first clip can reach it. */
export function useNoClipsOptions() {
  const view = useMotionStore(s => s.view);
  const asset = useMotionStore(s => s.asset);
  const setBoneMap = useMotionStore(s => s.setBoneMap);
  const setView = useAppStore(s => s.setView);
  const setAdding = useBuiltStore(s => s.setAdding);
  const model = view?.models.find(m => m.asset === asset) ?? null;
  return {
    model,
    retarget: () => setBoneMap(true),
    openInbox: () => setView('inbox'),
    addScript: () => {
      setView('board');
      setAdding(true);
    },
  };
}

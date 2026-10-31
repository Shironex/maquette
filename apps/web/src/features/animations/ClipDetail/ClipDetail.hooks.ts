import { useEffect, useState } from 'react';
import type { AnimationItem, ClipUse } from '@/ipc/client';
import { riggedModels } from '@/state/retarget';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRetargetStore } from '@/state/use-retarget-store';

/** Use on model for the open project, and an update for a model that baked an older take. */
export function useClipDetail(item: AnimationItem) {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const motion = useMotionStore(s => s.view);
  const open = useRetargetStore(s => s.open);
  const load = useMotionStore(s => s.load);
  const [trying, setTrying] = useState(false);
  const projectId = project?.id ?? null;
  // The library opens without Motion ever having read the project's models.
  useEffect(() => {
    if (projectId && useMotionStore.getState().view?.project !== projectId) void load(projectId);
  }, [projectId, load]);
  const first =
    motion && motion.project === project?.id ? (riggedModels(motion.models)[0] ?? null) : null;
  return {
    project,
    first,
    trying,
    toggleTry: () => setTrying(t => !t),
    useOnModel: () => {
      if (project)
        void open({ project: project.id, asset: first?.asset ?? null, clips: [item.clip.id] });
    },
    update: (use: ClipUse) =>
      void open({
        project: use.project,
        asset: use.asset,
        clips: [item.clip.id],
        step: 'settings',
      }),
  };
}

import { useEffect, useState } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useProjectTrashStore } from '@/state/use-project-trash-store';
import { useProjectsStore } from '@/state/use-projects-store';

/** The project Remove project asks about, what runs in it and the remove itself. */
export function useRemoveProjectDialog() {
  const asking = useProjectTrashStore(s => s.asking);
  const busy = useProjectTrashStore(s => s.busy);
  const remove = useProjectTrashStore(s => s.remove);
  const cancel = useProjectTrashStore(s => s.cancel);
  const projects = useAppStore(s => s.workspace?.projects ?? []);
  const row = useProjectsStore(s => s.list.find(p => p.id === asking));
  const load = useProjectsStore(s => s.load);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setError(null);
    // The switcher's rows say what runs in each project.
    if (asking) void load();
  }, [asking, load]);

  const project = projects.find(p => p.id === asking);
  if (!asking || !project) return null;
  const activity = row?.activity;
  const running = Boolean(activity?.run || activity?.concepts || activity?.sends);
  // Work running in it, or a remove the core refused: asking again cannot
  // help until the dialog opens again with that work over.
  const refusedBecause =
    error ?? (running ? 'Work is running in this project. Remove it once that is over.' : null);
  return {
    project,
    busy,
    error,
    running,
    refusedBecause,
    repo: project.siteRepoName ?? null,
    game: project.target === 'game',
    confirm: async () => setError(await remove(project.id)),
    cancel,
  };
}

import { useEffect, useState } from 'react';
import type { ProjectManifest } from '@/ipc/client';
import { waitingByHand } from '@/state/batch';
import { isActive } from '@/state/providers';
import type { BoardLayout } from '@/state/board-list';
import { useAppStore } from '@/state/use-app-store';
import { prefsOf, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** The Board's title row: the project, the By hand and 3D queue counts and Add asset. */
export function useBoardHeader() {
  const project = useAppStore(s => s.workspace?.project);
  const setProject = useAppStore(s => s.setProject);
  const jobs = useProvidersStore(s => s.jobs);
  const queueOpen = useProvidersStore(s => s.queueOpen);
  const setQueueOpen = useProvidersStore(s => s.setQueueOpen);
  const setChecklistOpen = useProvidersStore(s => s.setChecklistOpen);
  const byProject = useBoardPrefsStore(s => s.byProject);
  const updatePrefs = useBoardPrefsStore(s => s.update);
  const [adding, setAdding] = useState(false);

  // The add form belongs to the project it was opened in.
  useEffect(() => {
    setAdding(false);
  }, [project?.id]);

  const activeJobs = jobs.filter(j => isActive(j.state)).length;
  const byHand = jobs.some(j => j.provider === 'by-hand');
  const toMake = waitingByHand(jobs).length;
  const layout = prefsOf(byProject, project?.id).layout;
  const setLayout = (next: BoardLayout) => {
    if (project) updatePrefs(project.id, { layout: next });
  };
  const added = (next: ProjectManifest) => {
    setProject(next);
    setAdding(false);
  };

  return {
    project,
    layout,
    setLayout,
    adding,
    setAdding,
    added,
    activeJobs,
    byHand,
    toMake,
    queueOpen,
    setQueueOpen,
    setChecklistOpen,
  };
}

import { useEffect } from 'react';
import { useWait } from '@/lib/waiting';
import { isGame } from '@/state/game';
import { shownInspectorTab } from '@/state/inspector-tabs';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/**
 * The open project and its measured models, loaded for the project and
 * again after each finished run, which changes the shipped models.
 */
export function useInspectorView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const view = useInspectorStore(s => s.view);
  const loading = useInspectorStore(s => s.loading);
  const error = useInspectorStore(s => s.error);
  const load = useInspectorStore(s => s.load);
  const tab = useInspectorStore(s => s.tab);
  const projectId = project?.id ?? null;
  const lastRun = usePipelineStore(s => s.view?.runs[0]?.finishedAtMs ?? null);

  useEffect(() => {
    if (projectId) void load(projectId);
    // A finished run changes the shipped models, so it reloads too.
  }, [projectId, lastRun, load]);

  const estimate = useWait(
    projectId ? `inspector:${projectId}` : null,
    view === null && !error,
    Boolean(error)
  );

  return {
    project,
    view,
    loading,
    error,
    load,
    estimate,
    tab: shownInspectorTab(tab, isGame(project)),
  };
}

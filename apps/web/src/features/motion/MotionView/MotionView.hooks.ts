import { useEffect } from 'react';
import { moves } from '@/state/motion';
import { stageViews } from '@/state/pipeline';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/**
 * The open project's motion, loaded when the project or its models change.
 * The shell reloads it when an agent changes the layout.
 */
export function useMotionView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const latest = usePipelineStore(s => s.view?.runs[0]);
  const view = useMotionStore(s => s.view);
  const loading = useMotionStore(s => s.loading);
  const error = useMotionStore(s => s.error);
  const load = useMotionStore(s => s.load);
  const draft = useMotionStore(s => s.draft);
  const tab = useMotionStore(s => s.tab);
  const asset = useMotionStore(s => s.asset);
  const projectId = project?.id ?? null;
  const versions = (project?.assets ?? []).map(a => a.versions.length).join(',');
  useEffect(() => {
    if (projectId) void load(projectId);
  }, [projectId, versions, load]);
  const current = view && view.project === projectId ? view : null;
  const model = current?.models.find(m => m.asset === asset) ?? null;
  return {
    project,
    stages: stageViews(project?.assets ?? [], latest),
    view: current,
    loading,
    error,
    retry: () => (projectId ? void load(projectId) : undefined),
    moving: current ? current.models.filter(m => moves(m, draft)).length : 0,
    model,
    tab,
    hasStates: Boolean(model && draft.states[model.asset]),
  };
}

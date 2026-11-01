import { useWait } from '@/lib/waiting';
import { useEffect } from 'react';
import { moves } from '@/state/motion';
import { stageViews } from '@/state/pipeline';
import { useAppStore } from '@/state/use-app-store';
import { useBakedStore } from '@/state/use-baked-store';
import { isGame } from '@/state/game';
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
  const loadBaked = useBakedStore(s => s.load);
  useEffect(() => {
    if (!projectId) return;
    void load(projectId);
    void loadBaked(projectId);
  }, [projectId, versions, load, loadBaked]);
  const current = view && view.project === projectId ? view : null;
  const model = current?.models.find(m => m.asset === asset) ?? null;
  const estimate = useWait(
    projectId && `motion:${projectId}`,
    Boolean(projectId) && !current && !error,
    Boolean(error)
  );
  return {
    project,
    stages: stageViews(project?.assets ?? [], latest, project?.target),
    view: current,
    estimate,
    loading,
    error,
    retry: () => (projectId ? void load(projectId) : undefined),
    moving: current ? current.models.filter(m => moves(m, draft)).length : 0,
    model,
    // A game has no States tab.
    tab: isGame(project) ? 'clips' : tab,
    hasStates: Boolean(model && draft.states[model.asset]),
  };
}

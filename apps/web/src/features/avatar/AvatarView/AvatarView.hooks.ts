import { useEffect } from 'react';
import { buildAction, characters } from '@/state/avatar';
import { stageViews } from '@/state/pipeline';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/**
 * The open project's character assets and the one the view shows: the one
 * Motion opened, else the first. Reloads when the project or the asset's
 * versions change.
 */
export function useAvatarView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const latest = usePipelineStore(s => s.view?.runs[0]);
  const stored = useAvatarStore(s => s.view);
  const draft = useAvatarStore(s => s.draft);
  const asset = useAvatarStore(s => s.asset);
  const ownProject = useAvatarStore(s => s.project);
  const loading = useAvatarStore(s => s.loading);
  const error = useAvatarStore(s => s.error);
  const building = useAvatarStore(s => s.building);
  const tab = useAvatarStore(s => s.tab);
  const list = characters(project);
  const projectId = project?.id ?? null;
  const known = list.some(a => a.id === asset);
  const first = list[0]?.id ?? null;
  const versions = list.map(a => a.versions.length).join(',');
  useEffect(() => {
    if (!projectId || (ownProject === projectId && known)) return;
    useAvatarStore.setState({ project: projectId, asset: null });
    useAvatarStore.getState().select(first);
  }, [projectId, ownProject, known, first]);
  useEffect(() => {
    if (projectId) void useAvatarStore.getState().load();
  }, [projectId, versions]);
  const view = stored && stored.project === projectId && stored.asset === asset ? stored : null;
  return {
    project,
    stages: stageViews(project?.assets ?? [], latest, project?.target),
    count: list.length,
    view,
    tab,
    loading,
    error,
    retry: () => void useAvatarStore.getState().load(),
    building,
    action: buildAction(view, draft),
    build: () => void useAvatarStore.getState().build(),
  };
}

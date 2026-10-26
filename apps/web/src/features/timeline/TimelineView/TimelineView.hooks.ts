import { useEffect } from 'react';
import { placeLine } from '@/state/timeline-checks';
import { useAppStore } from '@/state/use-app-store';
import { useTimelineStore } from '@/state/use-timeline-store';
import { usePlayhead } from '../TimelineTracks/TimelineTracks.hooks';

/**
 * The open project's timeline, loaded when the project or its models
 * change. The shell reloads it when an agent changes the layout.
 */
export function useTimelineView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const setView = useAppStore(s => s.setView);
  const view = useTimelineStore(s => s.view);
  const loading = useTimelineStore(s => s.loading);
  const error = useTimelineStore(s => s.error);
  const saving = useTimelineStore(s => s.saving);
  const saveError = useTimelineStore(s => s.saveError);
  const load = useTimelineStore(s => s.load);
  const playhead = usePlayhead();
  const projectId = project?.id ?? null;
  const versions = (project?.assets ?? []).map(a => a.versions.length).join(',');
  useEffect(() => {
    if (projectId) void load(projectId);
  }, [projectId, versions, load]);
  const current = view && view.project === projectId ? view : null;
  return {
    project,
    view: current,
    loading,
    error,
    retry: () => (projectId ? void load(projectId) : undefined),
    saveLine: saving
      ? 'Saving to layout.json'
      : saveError
        ? 'Not saved. Edit again to retry'
        : 'Saved to layout.json as you go',
    place: current
      ? `Scroll ${Math.round(playhead.percent)}% · camera ${placeLine(playhead.at, current.stops)}`
      : '',
    openPage: () => setView('page'),
  };
}

import type { BoardLayout } from '@/state/board-list';
import { gameOf } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { prefsOf, useBoardPrefsStore } from '@/state/use-board-prefs-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The Board's title and its layout switch. */
export function useBoardHeader() {
  const project = useAppStore(s => s.workspace?.project);
  const notShipped = usePipelineStore(s =>
    s.view && s.view.project === project?.id && !s.view.active
      ? (s.view.runs[0]?.notShipped ?? null)
      : null
  );
  const byProject = useBoardPrefsStore(s => s.byProject);
  const updatePrefs = useBoardPrefsStore(s => s.update);
  const setLayout = (next: BoardLayout) => {
    if (project) updatePrefs(project.id, { layout: next });
  };
  return {
    project,
    notShipped,
    game: gameOf(project),
    layout: prefsOf(byProject, project?.id).layout,
    setLayout,
  };
}

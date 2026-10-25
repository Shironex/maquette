import type { ProjectManifest, ProviderJob, WorkspaceSnapshot } from '@/ipc/client';
import type { useBoardSelection, useCardActions } from './BoardView/BoardView.hooks';

export type BoardSelection = ReturnType<typeof useBoardSelection>;
export type CardActions = ReturnType<typeof useCardActions>;

/** What every card on the Board reads: the project, the jobs and the Board's own state. */
export interface BoardEnv {
  workspace: WorkspaceSnapshot | null;
  project: ProjectManifest | null | undefined;
  jobs: readonly ProviderJob[];
  otherProjects: readonly ProjectManifest[];
  /** The By hand job under a native file drag, if any. */
  dropJob: string | null;
  selection: BoardSelection;
  actions: CardActions;
}

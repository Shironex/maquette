import type { Stage } from '@/ipc/client';
import { stageLabels } from './game';
import { useAppStore } from './use-app-store';

/** The stage names for the open project: Export in place of Web for a game. */
export function useStageLabels(): Record<Stage, string> {
  return stageLabels(useAppStore(s => s.workspace?.project?.target));
}

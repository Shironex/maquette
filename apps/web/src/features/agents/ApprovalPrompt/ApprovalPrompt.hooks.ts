import { isGame } from '@/state/game';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';

/**
 * The approvals to float over the current view; Settings > Agents lists them
 * in place, and so does the Pipeline view's Ship diff for the open game.
 */
export function useFloatingApprovals() {
  const approvals = useAgentsStore(s => s.status?.approvals);
  const view = useAppStore(s => s.view);
  const project = useAppStore(s => s.workspace?.project);
  if (view === 'settings') return undefined;
  if (view !== 'pipeline' || !isGame(project)) return approvals;
  return approvals?.filter(a => a.kind !== 'ship' || a.project !== project?.id);
}

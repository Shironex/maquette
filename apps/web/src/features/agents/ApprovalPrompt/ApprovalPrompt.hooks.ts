import { isGame } from '@/state/game';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';

/**
 * The approvals to float over the current view; Settings > Agents lists them
 * in place, and so do the Pipeline view's Ship diff for the open game and
 * the Avatar view for the rest pose of its asset.
 */
export function useFloatingApprovals() {
  const approvals = useAgentsStore(s => s.status?.approvals);
  const view = useAppStore(s => s.view);
  const project = useAppStore(s => s.workspace?.project);
  const avatar = useAvatarStore(s => s.asset);
  if (view === 'settings') return undefined;
  // The Avatar view asks about the rest pose of the asset it shows itself.
  if (view === 'avatar') {
    return approvals?.filter(a => a.kind !== 'rest-pose' || a.asset !== avatar);
  }
  if (view !== 'pipeline' || !isGame(project)) return approvals;
  return approvals?.filter(a => a.kind !== 'ship' || a.project !== project?.id);
}

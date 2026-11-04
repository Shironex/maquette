import { isGame } from '@/state/game';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { useRigStore } from '@/state/use-rig-store';

/**
 * The approvals to float over the current view; Settings > Agents lists them
 * in place, and so do the Pipeline view's Ship diff for the open game, the
 * Avatar view for the rest pose, the weights and the spring chains of its
 * asset, and the Rig doctor for the case it shows (Add and Discard answer it).
 */
export function useFloatingApprovals() {
  const approvals = useAgentsStore(s => s.status?.approvals);
  const view = useAppStore(s => s.view);
  const project = useAppStore(s => s.workspace?.project);
  const avatar = useAvatarStore(s => s.asset);
  const rigCase = useRigStore(s => s.record?.id);
  if (view === 'settings') return undefined;
  // The Avatar view asks about each step of the asset it shows itself.
  if (view === 'avatar') {
    return approvals?.filter(
      a =>
        (a.kind !== 'rest-pose' && a.kind !== 'reweight' && a.kind !== 'springs') ||
        a.asset !== avatar
    );
  }
  // The Rig doctor's own bar is the answer to its case's card.
  if (view === 'inbox' && rigCase) {
    return approvals?.filter(a => a.kind !== 'rig-doctor' || a.job !== rigCase);
  }
  if (view !== 'pipeline' || !isGame(project)) return approvals;
  return approvals?.filter(a => a.kind !== 'ship' || a.project !== project?.id);
}

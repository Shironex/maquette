import { useSyncExternalStore } from 'react';
import { create } from 'zustand';
import { isGame } from '@/state/game';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { useRigStore } from '@/state/use-rig-store';

/** The width of the tray the approval cards dock in, beside the view. */
export const APPROVAL_TRAY_WIDTH = 360;

/** The width of the folded tray: a rail with the count. */
export const APPROVAL_RAIL_WIDTH = 44;

/**
 * A window this wide keeps room for a view beside the open tray, so the tray
 * starts open; a narrower one starts with it folded to the rail.
 */
const WIDE = '(min-width: 1400px)';

interface ApprovalTrayStore {
  /** The person's choice this session; null follows the window's width. */
  folded: boolean | null;
  setFolded: (folded: boolean) => void;
}

export const useApprovalTrayStore = create<ApprovalTrayStore>(set => ({
  folded: null,
  setFolded: folded => set({ folded }),
}));

const hasMedia = () => typeof window !== 'undefined' && typeof window.matchMedia === 'function';

function onWidth(change: () => void) {
  if (!hasMedia()) return () => undefined;
  const query = window.matchMedia(WIDE);
  query.addEventListener('change', change);
  return () => query.removeEventListener('change', change);
}

// Without media queries (tests) the window counts as wide.
const isWide = () => !hasMedia() || window.matchMedia(WIDE).matches;

/** Whether the approval tray is folded to its rail, its width, and the toggles. */
export function useApprovalTray() {
  const chosen = useApprovalTrayStore(s => s.folded);
  const setFolded = useApprovalTrayStore(s => s.setFolded);
  const wide = useSyncExternalStore(onWidth, isWide);
  const folded = chosen ?? !wide;
  return {
    folded,
    width: folded ? APPROVAL_RAIL_WIDTH : APPROVAL_TRAY_WIDTH,
    fold: () => setFolded(true),
    unfold: () => setFolded(false),
  };
}

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

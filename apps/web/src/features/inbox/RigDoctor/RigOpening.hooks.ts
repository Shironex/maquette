import { useRigStore } from '@/state/use-rig-store';

/** The way back from a case: the view it was opened from. */
export function useRigCrumbs() {
  const leave = useRigStore(s => s.leave);
  const from = useRigStore(s => s.from);
  return { leave, back: from === 'board' ? 'Board' : 'Inbox' };
}

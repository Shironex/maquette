import { caseLine } from '@/state/rig';
import { useRigStore } from '@/state/use-rig-store';
import { useEffect } from 'react';
import { useWait } from '@/lib/waiting';
import { firstToReview } from '@/state/inbox';
import { filterItems, triageGroup } from '@/state/triage';
import { useInboxStore } from '@/state/use-inbox-store';
import { useTriageStore } from '@/state/use-triage-store';

/**
 * The inbox listing, the item on show (the picked one, else the first to
 * review) and the layout. The discarded models are read again whenever the
 * inbox changes, so a discard shows up under Recently discarded.
 */
export function useInboxView() {
  const { snapshot, selected, loading, loadFailed } = useInboxStore();
  const { layout, setLayout, filter, loadDiscarded } = useTriageStore();
  const items = snapshot?.items ?? [];
  // The list's filter decides too: a model it hides is not the one on show.
  const kept = filterItems(items, filter);
  const item =
    kept.find(i => i.id === selected) ??
    firstToReview(kept) ??
    items.find(i => i.id === selected) ??
    null;
  useEffect(() => {
    void loadDiscarded();
  }, [snapshot, loadDiscarded]);
  const reading = snapshot === null && loading;
  const estimate = useWait('inbox', reading, loadFailed);
  return {
    snapshot,
    reading,
    estimate,
    items,
    item,
    layout,
    setLayout,
    replaced: items.filter(i => triageGroup(i) === 'replaced'),
  };
}

/** The rig doctor case on show, and the way back to one that is still open. */
export function useRigCases() {
  const rigCase = useRigStore(s => s.record);
  const rigOpening = useRigStore(s => s.opening);
  const open = useRigStore(s => s.open);
  const load = useRigStore(s => s.load);
  const showCase = useRigStore(s => s.showCase);
  useEffect(() => void load(), [load]);
  const going = open[0];
  const more = open.length > 1 ? ` and ${open.length - 1} more` : '';
  return {
    rigCase,
    rigOpening,
    going,
    goingLabel: going ? `${caseLine(going)}${more}` : '',
    showCase,
  };
}

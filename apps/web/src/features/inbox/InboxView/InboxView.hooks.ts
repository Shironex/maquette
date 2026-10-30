import { useEffect } from 'react';
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
  const { snapshot, selected } = useInboxStore();
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
  return {
    snapshot,
    items,
    item,
    layout,
    setLayout,
    replaced: items.filter(i => triageGroup(i) === 'replaced'),
  };
}

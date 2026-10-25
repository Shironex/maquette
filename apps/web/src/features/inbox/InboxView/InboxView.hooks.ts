import { useState } from 'react';
import { firstToReview } from '@/state/inbox';
import { batchRedoPlan } from '@/state/redo';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';

/** The inbox listing and the item on show: the picked one, else the first to review. */
export function useInboxView() {
  const { snapshot, selected, select, checked, check, clearChecked } = useInboxStore();
  const [menuFor, setMenuFor] = useState<string | null>(null);
  const openBatchRedo = useProvidersStore(s => s.openBatchRedo);
  const openRedo = useProvidersStore(s => s.openRedo);
  const items = snapshot?.items ?? [];
  const item = items.find(i => i.id === selected) ?? firstToReview(items);
  const plan = batchRedoPlan(items, checked);
  const redoChecked = () => {
    const [one] = plan.redo;
    if (!plan.project || !one) return;
    // One model opens the plain send panel; more go as one batch.
    if (plan.redo.length === 1) void openRedo(plan.project, one.asset, one.job);
    else void openBatchRedo(plan.project, plan.redo);
    clearChecked();
  };
  return {
    snapshot,
    items,
    item,
    select,
    checked,
    check,
    clearChecked,
    menuFor,
    toggleMenu: (id: string) => setMenuFor(m => (m === id ? null : id)),
    closeMenu: () => setMenuFor(null),
    redoBlocker: plan.blocker,
    redoChecked,
  };
}

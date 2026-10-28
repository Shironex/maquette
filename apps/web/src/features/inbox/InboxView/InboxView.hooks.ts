import { useRef, useState } from 'react';
import { useWindowedList } from '@maquette/ui';
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

/** An inbox row's height before it is measured. */
const ROW = 62;

/** The inbox list's scroller and the rows in view, windowed once the inbox is long. */
export function useInboxRows(count: number) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const shown = useWindowedList({ count, scrollRef, listRef, estimate: ROW, gap: 4 });
  return { scrollRef, listRef, shown };
}

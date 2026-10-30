import { useState } from 'react';
import { batchRedoPlan } from '@/state/redo';
import { acceptPlan, selectionNote } from '@/state/triage';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useTriageStore, type AcceptAfter } from '@/state/use-triage-store';

/**
 * The ticked models and what can be done with them together: the plan an
 * accept follows, the batch redo, and the discard that asks first.
 */
export function useSelection() {
  const { snapshot, checked, check, clearChecked } = useInboxStore();
  const jobs = useProvidersStore(s => s.jobs);
  const openRedo = useProvidersStore(s => s.openRedo);
  const openBatchRedo = useProvidersStore(s => s.openBatchRedo);
  const { working, acceptMany, discardMany } = useTriageStore();
  const [discarding, setDiscarding] = useState<readonly string[] | null>(null);
  const items = snapshot?.items ?? [];
  const plan = acceptPlan(items, checked, jobs);
  const redo = batchRedoPlan(items, checked);
  return {
    items,
    checked,
    check,
    /** Ticks or unticks every model in `ids`. */
    checkAll: (ids: readonly string[], on: boolean) => ids.forEach(id => check(id, on)),
    clearChecked,
    note: selectionNote(items, checked),
    plan,
    working,
    redoBlocker: redo.blocker,
    redoChecked: () => {
      const [one] = redo.redo;
      if (!redo.project || !one) return;
      // One model opens the plain send panel; more go as one batch.
      if (redo.redo.length === 1) void openRedo(redo.project, one.asset, one.job);
      else void openBatchRedo(redo.project, redo.redo);
      clearChecked();
    },
    accept: (after?: AcceptAfter) => void acceptMany(plan.ids, after),
    /** The models the discard dialog asks about, or null while it is closed. */
    discarding,
    askDiscard: (ids: readonly string[] = checked) => setDiscarding(ids),
    closeDiscard: () => setDiscarding(null),
    confirmDiscard: async () => {
      if (!discarding) return;
      await discardMany(discarding);
      setDiscarding(null);
    },
  };
}

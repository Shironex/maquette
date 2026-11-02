import { useMemo, useState } from 'react';
import type { PlanSummary } from '@/ipc/client';
import { useNow } from '@/lib/useNow';
import { trimText } from '@/state/plan-text';
import { byEdited, findPlans } from '@/state/plans';
import { usePlansStore } from '@/state/use-plans-store';

/** How many canvases the Recent row shows. */
export const RECENT = 4;

/**
 * The canvas list: the search, the Recent row, inline rename, and which
 * dialog is open (New canvas or Remove).
 */
export function usePlanList(list: readonly PlanSummary[]) {
  const openCanvas = usePlansStore(s => s.openCanvas);
  const rename = usePlansStore(s => s.rename);
  const [query, setQuery] = useState('');
  const [making, setMaking] = useState(false);
  const [removing, setRemoving] = useState<PlanSummary | null>(null);
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  const now = useNow(30_000);
  const found = useMemo(() => byEdited(findPlans(list, query)), [list, query]);
  const recent = useMemo(
    () => (query.trim() ? [] : byEdited(list.filter(p => !p.problem)).slice(0, RECENT)),
    [list, query]
  );
  return {
    now,
    query,
    setQuery,
    found,
    recent,
    making,
    startNew: () => setMaking(true),
    closeNew: () => setMaking(false),
    removing,
    askRemove: (plan: PlanSummary) => setRemoving(plan),
    closeRemove: () => setRemoving(null),
    renaming,
    startRename: (plan: PlanSummary) => setRenaming({ id: plan.id, name: plan.name }),
    typeName: (name: string) => setRenaming(r => (r ? { ...r, name } : r)),
    cancelRename: () => setRenaming(null),
    commitRename: async () => {
      if (!renaming) return;
      const plan = list.find(p => p.id === renaming.id);
      const name = trimText(renaming.name);
      if (!plan || !name || name === plan.name) {
        setRenaming(null);
        return;
      }
      if (await rename(renaming.id, name)) setRenaming(null);
    },
    open: (id: string) => void openCanvas(id),
  };
}

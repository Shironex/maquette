import type { ProjectManifest } from '@/ipc/client';
import { useNow } from '@/lib/useNow';
import { creditTotals, filterSpends, ledgerProjects } from '@/state/credits';
import { useAppStore } from '@/state/use-app-store';
import { useCreditsStore } from '@/state/use-credits-store';

const NO_PROJECTS: readonly ProjectManifest[] = [];

/** The Credits sheet's data: the ledger as filtered, its totals and the filter choices. */
export function useCreditsSheet() {
  const store = useCreditsStore();
  const projects = useAppStore(s => s.workspace?.projects) ?? NO_PROJECTS;
  const now = useNow(30_000);
  const spends = store.ledger ? filterSpends(store.ledger.spends, store.filter) : [];
  const nameOf = (id: string) => projects.find(p => p.id === id)?.name ?? id;
  const ids = new Set([...projects.map(p => p.id), ...ledgerProjects(store.ledger?.spends ?? [])]);
  const projectOptions = [
    { value: '', label: 'All projects' },
    ...[...ids].map(id => ({ value: id, label: nameOf(id) })),
  ];
  return {
    ...store,
    now,
    spends,
    totals: creditTotals(spends, now),
    nameOf,
    projectOptions,
    showProject: !store.filter.project,
  };
}

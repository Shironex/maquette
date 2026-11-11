import { useEffect } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useGripLibraryStore } from '@/state/use-grip-library-store';

/** The grip library, read when the part opens, and the grip on show: the selected one, else the first. */
export function useLibraryGrips() {
  const list = useGripLibraryStore(s => s.list);
  const error = useGripLibraryStore(s => s.error);
  const selected = useGripLibraryStore(s => s.selected);
  const filter = useGripLibraryStore(s => s.filter);
  const load = useGripLibraryStore(s => s.load);
  const root = useAppStore(s => s.workspace?.root);
  useEffect(() => {
    void load();
  }, [load, root]);
  const { select, setFilter } = useGripLibraryStore.getState();
  return {
    list,
    error,
    filter,
    summary: list?.find(g => g.id === selected) ?? list?.[0] ?? null,
    select,
    setFilter,
    retry: () => void load(),
  };
}

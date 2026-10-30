import { useEffect } from 'react';
import { useGridOrListWindow } from '@/lib/useGridOrListWindow';
import type { LibraryLayout } from '@/state/library';
import { filterItems, libraryStats, sortItems } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore } from '@/state/use-library-store';
import { useLibraryQuickView } from '@/state/use-quick-view-store';

/** The library's entries as the filters and sort leave them, and the one on show. */
export function useLibraryView() {
  const store = useLibraryStore();
  const { view, filters, sort, selected, load } = store;
  const workspace = useAppStore(s => s.workspace);
  const quickView = useLibraryQuickView();
  const items = view?.items ?? [];
  const shown = sortItems(filterItems(items, filters), sort);
  const item = items.find(i => i.entry.id === selected) ?? shown[0] ?? null;

  useEffect(() => {
    void load();
  }, [load, workspace?.root]);

  return {
    ...store,
    workspace,
    items,
    shown,
    item,
    stats: libraryStats(items),
    loaded: view !== null,
    quickView,
  };
}

/** A model card's and row's height before they are measured, and the grid's `minmax(172px, 1fr)`. */
const SIZES = { card: 236, row: 58, column: 172 };

/** The models' scroller and the models in view, windowed once the library is long. */
export function useModelsWindow(count: number, layout: LibraryLayout) {
  return useGridOrListWindow(count, layout === 'grid', SIZES);
}

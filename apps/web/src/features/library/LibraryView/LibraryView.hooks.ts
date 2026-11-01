import { useWait } from '@/lib/waiting';
import { useEffect, type MouseEvent } from 'react';
import { useGridOrListWindow } from '@/lib/useGridOrListWindow';
import type { LibraryLayout } from '@/state/library';
import { UNTAGGED, filterItems, libraryStats, sortItems, tagCounts } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useAddManyStore } from '@/state/use-add-many-store';
import { useLibrarySelectionStore } from '@/state/use-library-selection-store';
import { useLibraryStore } from '@/state/use-library-store';
import { usePackStore } from '@/state/use-pack-store';
import { useLookAlikesStore } from '@/state/use-look-alikes-store';
import { useLibraryQuickView } from '@/state/use-quick-view-store';
import type { ModelFlag } from './LibraryView.types';

const count = (n: number) => `${n} ${n === 1 ? 'model' : 'models'}`;

/** The library's entries as the filters and sort leave them, the one on show and the pairs to look at. */
export function useLibraryView() {
  const store = useLibraryStore();
  const { view, filters, sort, selected, load } = store;
  const workspace = useAppStore(s => s.workspace);
  const quickView = useLibraryQuickView();
  const looks = useLookAlikesStore();
  const scan = looks.scan;
  const showAddMany = useAddManyStore(s => s.show);
  const addManyOpen = useAddManyStore(s => s.open);
  const showPack = usePackStore(s => s.show);
  const items = view?.items ?? [];
  const collections = view?.collections ?? [];
  const shown = sortItems(filterItems(items, filters, collections), sort);
  const item = items.find(i => i.entry.id === selected) ?? shown[0] ?? null;
  const loaded = view !== null;
  const readFailed = Boolean(store.error);
  const estimate = useWait('library', !loaded && !readFailed, readFailed);

  useEffect(() => {
    void load();
  }, [load, workspace?.root]);
  // The pairs are read once the library is, and again after it changes size.
  useEffect(() => {
    if (loaded) void scan();
  }, [loaded, items.length, scan]);

  const flags = new Map<string, ModelFlag>();
  for (const p of looks.report?.pairs ?? []) {
    for (const id of [p.a, p.b]) if (!flags.has(id)) flags.set(id, p.kind);
  }
  const collection = collections.find(c => c.id === filters.collection);
  const filtered = shown.length !== items.length;

  return {
    ...store,
    estimate,
    workspace,
    items,
    collections,
    shown,
    item,
    stats: libraryStats(items),
    loaded,
    quickView,
    flags,
    report: looks.report,
    known: tagCounts(items).map(([tag]) => tag),
    untagged: items.filter(i => i.entry.tags.length === 0).length,
    title: collection?.name ?? (filtered ? 'Models found' : 'All models'),
    count: filtered ? `${shown.length} of ${count(items.length)}` : count(items.length),
    showUntagged: () => store.setFilters({ tag: UNTAGGED }),
    addManyOpen,
    openAddMany: () => showAddMany(workspace?.project?.id ?? null),
    pack: {
      tip: collection
        ? `Writes every model in ${collection.name} as one pack, checked in Blender and zipped`
        : 'Writes the models in view as one pack, checked in Blender and zipped; pick some first to pack only those',
      start: () =>
        void (collection
          ? showPack(
              { from: 'collection', collection: collection.id },
              { collection: collection.name }
            )
          : showPack({ from: 'entries', entries: shown.map(i => i.entry.id) })),
    },
  };
}

/**
 * The bulk selection over the models in view: a select box picks one, Shift
 * a range, Cmd or Ctrl and a click on a card toggles it, and while a
 * selection is under way any click on a card does. Escape clears it.
 */
export function useModelSelection(order: readonly string[], onPick: (id: string) => void) {
  const { selected, check, set, clear } = useLibrarySelectionStore();
  const picked = new Set(selected);
  const selecting = selected.length > 0;

  useEffect(() => {
    if (!selecting) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !document.querySelector('[aria-modal="true"], [role="dialog"]'))
        clear();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [selecting, clear]);

  return {
    selected,
    selecting,
    isPicked: (id: string) => picked.has(id),
    check: (id: string) => (on: boolean, range: boolean) => check(id, on, range, order),
    click: (id: string) => (e?: MouseEvent) => {
      if (e && (e.shiftKey || e.metaKey || e.ctrlKey || selecting)) {
        check(id, !picked.has(id), e.shiftKey, order);
        return;
      }
      onPick(id);
    },
    selectAll: () => set(order),
    clear,
  };
}

/** A model card's and row's height before they are measured, and the grid's `minmax(172px, 1fr)`. */
const SIZES = { card: 252, row: 58, column: 172 };

/** The models' scroller and the models in view, windowed once the library is long. */
export function useModelsWindow(count: number, layout: LibraryLayout) {
  return useGridOrListWindow(count, layout !== 'list', SIZES);
}

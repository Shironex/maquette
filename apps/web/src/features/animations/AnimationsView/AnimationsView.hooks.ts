import { useEffect, useRef } from 'react';
import { useGridColumns, useWindowedList } from '@maquette/ui';
import type { AnimationLayout } from '@/state/animations';
import { animationStats, filterClips } from '@/state/animations';
import { useAppStore } from '@/state/use-app-store';
import { useAnimationsStore } from '@/state/use-animations-store';

/** The library's clips as the filters leave them, and the one on show. */
export function useAnimationsView() {
  const store = useAnimationsStore();
  const { view, filters, selected, load } = store;
  const workspace = useAppStore(s => s.workspace);
  const items = view?.items ?? [];
  const shown = filterClips(items, filters);
  const item = items.find(i => i.clip.id === selected) ?? shown[0] ?? null;

  useEffect(() => {
    void load();
  }, [load, workspace?.root]);

  return {
    ...store,
    items,
    shown,
    item,
    stats: animationStats(items, view?.diskBytes ?? 0),
    loaded: view !== null,
    /** The library folder as the person knows it, `~/Maquette/library/animations`. */
    dir:
      view && workspace ? view.dir.replace(workspace.root, workspace.display) : (view?.dir ?? null),
  };
}

/** A grid card's and a list row's height before they are measured. */
const CARD = 214;
const ROW = 58;
/** The grid's narrowest column and its gaps (`minmax(164px, 1fr)`, `gap-3`). */
const COLUMN = 164;
const GRID_GAP = 12;

/** The clips' scroller and the clips in view, windowed once the library is long. */
export function useClipsWindow(count: number, layout: AnimationLayout) {
  const scrollRef = useRef<HTMLElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const grid = layout === 'grid';
  const columns = useGridColumns(listRef, COLUMN, GRID_GAP);
  const shown = useWindowedList({
    count,
    scrollRef,
    listRef,
    estimate: grid ? CARD : ROW,
    gap: grid ? GRID_GAP : 4,
    columns: grid ? columns : 1,
  });
  return { scrollRef, listRef, shown };
}

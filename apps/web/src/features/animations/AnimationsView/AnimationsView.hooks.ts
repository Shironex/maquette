import { useEffect } from 'react';
import { useGridOrListWindow } from '@/lib/useGridOrListWindow';
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

/** A clip card's and row's height before they are measured, and the grid's `minmax(164px, 1fr)`. */
const SIZES = { card: 214, row: 58, column: 164 };

/** The clips' scroller and the clips in view, windowed once the library is long. */
export function useClipsWindow(count: number, layout: AnimationLayout) {
  return useGridOrListWindow(count, layout === 'grid', SIZES);
}

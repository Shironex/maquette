import { useEffect } from 'react';
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

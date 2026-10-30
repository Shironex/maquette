/**
 * The models picked in the Library for a bulk action: Tag, Add to
 * collection, Use in project and Remove act on it, and so can any later
 * action that takes many models (it holds entry ids only, in the order the
 * view showed them). Shift extends a range from the last model clicked.
 */
import { create } from 'zustand';
import { nextSelection } from './batch';

interface LibrarySelectionStore {
  /** Entry ids, in the view's order. */
  selected: string[];
  /** The last model clicked, where a Shift range starts. */
  anchor: string | null;
  /** A model's select box: on or off, with `range` for Shift, over `order` as shown. */
  check: (id: string, on: boolean, range: boolean, order: readonly string[]) => void;
  /** Picks exactly these. */
  set: (ids: readonly string[]) => void;
  clear: () => void;
  /** Lets go of models that are no longer in the library. */
  keep: (existing: readonly string[]) => void;
}

export const useLibrarySelectionStore = create<LibrarySelectionStore>((set, get) => ({
  selected: [],
  anchor: null,
  check: (id, on, range, order) =>
    set(s => ({ selected: nextSelection(s.selected, order, id, on, range, s.anchor), anchor: id })),
  set: ids => set({ selected: [...new Set(ids)], anchor: null }),
  clear: () => set({ selected: [], anchor: null }),
  keep: existing => {
    const known = new Set(existing);
    const kept = get().selected.filter(id => known.has(id));
    if (kept.length !== get().selected.length) set({ selected: kept });
  },
}));

/**
 * The workspace's grip library: the list the Animations library's Grips,
 * the recipe editor's grip picker and its "Update to rN" offers read, kept
 * up to date by the grip library changed event. Also the Grips part's
 * selected grip and weapon filter, and the person's removals.
 * A removal sends the version the list read, so one that moved since is
 * refused rather than removing what the person did not see.
 */
import { create } from 'zustand';
import { commands, errorMessage, type LibraryGripSummary } from '@/ipc/client';

/** The Grips part's filter: every grip, or one weapon group. */
export type GripFilter = 'all' | 'swords' | 'polearms' | 'daggers' | 'shields' | 'empty';

interface GripLibraryStore {
  list: LibraryGripSummary[] | null;
  error: string | null;
  /** The grip the Grips part's side panel shows. */
  selected: string | null;
  filter: GripFilter;

  load: () => Promise<void>;
  /** Reloads when the list was read. */
  onChanged: () => void;
  select: (entry: string | null) => void;
  setFilter: (filter: GripFilter) => void;
  /** Removes one revision; what it was refused with, or null. */
  removeRevision: (entry: string, revision: number) => Promise<string | null>;
  /** Removes the grip with every revision; what it was refused with, or null. */
  remove: (entry: string) => Promise<string | null>;
}

/** Counts list reads, so an older answer never replaces a newer one. */
let reads = 0;

/** The grips `filter` leaves, in the list's order. */
export const filterGrips = (list: readonly LibraryGripSummary[], filter: GripFilter) =>
  filter === 'all' ? [...list] : list.filter(g => g.group === filter);

export const useGripLibraryStore = create<GripLibraryStore>((set, get) => {
  const versionOf = (entry: string) => get().list?.find(e => e.id === entry)?.version ?? null;
  /** Runs a removal, then reads the list again whatever it answered. */
  const removal = async (run: () => Promise<unknown>) => {
    try {
      await run();
      return null;
    } catch (error) {
      return errorMessage(error);
    } finally {
      await get().load();
    }
  };
  return {
    list: null,
    error: null,
    selected: null,
    filter: 'all',

    load: async () => {
      const read = ++reads;
      try {
        const list = await commands.gripLibraryList();
        if (read === reads) set({ list, error: null });
      } catch (error) {
        if (read === reads) set({ error: errorMessage(error) });
      }
    },

    onChanged: () => {
      if (get().list) void get().load();
    },

    select: selected => set({ selected }),
    setFilter: filter => set({ filter }),

    removeRevision: (entry, revision) =>
      removal(() => commands.gripLibraryRemoveRevision(entry, revision, versionOf(entry))),

    remove: async entry => {
      const refused = await removal(() => commands.gripLibraryRemove(entry, versionOf(entry)));
      if (!refused && get().selected === entry) set({ selected: null });
      return refused;
    },
  };
});

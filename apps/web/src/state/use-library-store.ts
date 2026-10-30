/**
 * The workspace's model library: its entries, the view's filters, and Add
 * to library, Use in project and Update from library. The library copies
 * models into projects; it never changes a project on its own, so every
 * update goes through the Update dialog's plan first.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type AddToLibrary,
  type LibraryView,
  type UpdatePlan,
} from '@/ipc/client';
import { NO_FILTERS, type LibraryFilters, type LibraryLayout, type LibrarySort } from './library';
import { useAppStore } from './use-app-store';
import { useLibrarySelectionStore } from './use-library-selection-store';

/** A project asset a dialog is about. */
export interface AssetRef {
  project: string;
  asset: string;
}

interface LibraryStore {
  view: LibraryView | null;
  error: string | null;
  filters: LibraryFilters;
  sort: LibrarySort;
  layout: LibraryLayout;
  /** The entry the side panel shows. */
  selected: string | null;
  /** The Add to library dialog: an asset, `'pick'` to choose one, or closed. */
  adding: AssetRef | 'pick' | null;
  /** The asset the Update dialog is about, and its plan once read. */
  updating: AssetRef | null;
  plan: UpdatePlan | null;
  planError: string | null;
  /** An add, use or update is on its way. */
  busy: boolean;

  load: () => Promise<void>;
  /** Reloads when the library was loaded. */
  onChanged: () => void;
  setFilters: (change: Partial<LibraryFilters>) => void;
  setSort: (sort: LibrarySort) => void;
  setLayout: (layout: LibraryLayout) => void;
  select: (entry: string | null) => void;
  openAdd: (target: AssetRef | 'pick') => void;
  closeAdd: () => void;
  /** Adds the model. Rejects with the reason it was refused. */
  add: (request: AddToLibrary) => Promise<void>;
  /** Copies an entry's version (the newest when null) into a project. */
  use: (entry: string, version: number | null, project: string) => Promise<void>;
  openUpdate: (target: AssetRef) => Promise<void>;
  closeUpdate: () => void;
  update: () => Promise<void>;
}

export const useLibraryStore = create<LibraryStore>((set, get) => ({
  view: null,
  error: null,
  filters: NO_FILTERS,
  sort: 'used',
  layout: 'grid',
  selected: null,
  adding: null,
  updating: null,
  plan: null,
  planError: null,
  busy: false,

  load: async () => {
    try {
      const view = await commands.libraryGet();
      const selected = get().selected;
      const kept = view.items.some(i => i.entry.id === selected) ? selected : null;
      set({ view, error: null, selected: kept });
      useLibrarySelectionStore.getState().keep(view.items.map(i => i.entry.id));
    } catch (error) {
      set({ error: errorMessage(error) });
    }
  },

  onChanged: () => {
    if (get().view) void get().load();
  },

  setFilters: change => set(s => ({ filters: { ...s.filters, ...change } })),
  setSort: sort => set({ sort }),
  setLayout: layout => set({ layout }),
  select: selected => set({ selected }),
  openAdd: adding => set({ adding }),
  closeAdd: () => set({ adding: null }),

  add: async request => {
    set({ busy: true });
    try {
      const added = await commands.libraryAdd(request);
      const { setProject, notify } = useAppStore.getState();
      setProject(added.project);
      notify(
        added.added
          ? { status: 'done', title: `Added ${added.entry.name} v${added.version} to the library` }
          : {
              status: 'info',
              title: `${added.entry.name} is already in the library`,
              detail: `The library has this model as v${added.version}; nothing was copied.`,
            }
      );
      set({ adding: null, selected: added.entry.id });
      await get().load();
    } finally {
      set({ busy: false });
    }
  },

  use: async (entry, version, project) => {
    const { setProject, notify } = useAppStore.getState();
    set({ busy: true });
    try {
      const used = await commands.libraryUse(entry, version, project);
      setProject(used.project);
      notify({
        status: 'done',
        title: `Added ${used.asset.name} to ${used.project.name}, no credits`,
        detail: used.note ?? undefined,
      });
      await get().load();
    } catch (error) {
      notify({ status: 'failed', title: 'Could not use the model', detail: errorMessage(error) });
    } finally {
      set({ busy: false });
    }
  },

  openUpdate: async target => {
    set({ updating: target, plan: null, planError: null });
    try {
      const plan = await commands.libraryUpdatePlan(target.project, target.asset);
      if (get().updating === target) set({ plan });
    } catch (error) {
      if (get().updating === target) set({ planError: errorMessage(error) });
    }
  },

  closeUpdate: () => set({ updating: null, plan: null, planError: null }),

  update: async () => {
    const target = get().updating;
    if (!target) return;
    const { setProject, notify } = useAppStore.getState();
    set({ busy: true });
    try {
      const done = await commands.libraryUpdate(target.project, target.asset);
      setProject(done.project);
      const dropped = done.plan.dropped.length;
      notify({
        status: 'done',
        title: `Updated ${done.plan.name} to library v${done.plan.to}`,
        detail: `It is v${done.plan.nextVersion} in the project${
          dropped
            ? `; ${dropped === 1 ? 'one setting' : `${dropped} settings`} no longer applied`
            : ''
        }.`,
      });
      set({ updating: null, plan: null });
      await get().load();
    } catch (error) {
      set({ planError: errorMessage(error) });
    } finally {
      set({ busy: false });
    }
  },
}));

/**
 * The workspace's recipe library: the list the Animations library's
 * Recipes section and the Recipes tab's "Library has rN" notes read, kept
 * up to date by the recipe library changed event, with the starter recipes
 * built into the app. Also the section's selected entry or starter, Import
 * recipe, Save a starter to the library, Use in project, Export and the
 * removals.
 * A removal sends the version the list read, so one that moved since is
 * refused rather than removing what the person did not see.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type ExportSource,
  type LibraryRecipeSummary,
  type RecipeImported,
  type RecipeStarter,
} from '@/ipc/client';
import { useAppStore } from './use-app-store';

/** The revision Use in project copies (none for a starter), and the name it starts with. */
export interface UseTarget {
  entry: string;
  revision: number | null;
  name: string;
}

/** The last recipe added to the library: imported from a file, or a starter saved there. */
export type LibraryAdded = RecipeImported & { starter: boolean };

interface RecipeLibraryStore {
  list: LibraryRecipeSummary[] | null;
  /** The starter recipes built into the app, read with the list. */
  starters: RecipeStarter[] | null;
  error: string | null;
  /** The entry (or starter id) the Animations library's side panel shows. */
  selected: string | null;
  /** The last import or starter save into the library and what to know about it. */
  imported: LibraryAdded | null;
  /** The starter being saved to the library. */
  copying: string | null;
  importing: boolean;
  importError: string | null;
  /** The Use in project sheet is open on this revision. */
  using: UseTarget | null;

  load: () => Promise<void>;
  /** Reloads when the list was read. */
  onChanged: () => void;
  select: (entry: string | null) => void;
  /** Picks a `.recipe.json` with the system dialog and imports it as a new library recipe. */
  importFile: () => Promise<void>;
  /** Saves a starter to the library as a new recipe at r1 the person can edit. */
  copyStarter: (id: string) => Promise<void>;
  dismissImported: () => void;
  openUse: (target: UseTarget) => void;
  closeUse: () => void;
  /** Asks where to save the recipe file and writes it there; says where it went. */
  exportRecipe: (source: ExportSource, name: string) => Promise<void>;
  /** Removes one revision; what it was refused with, or null. */
  removeRevision: (entry: string, revision: number) => Promise<string | null>;
  /** Removes the entry with every revision; what it was refused with, or null. */
  remove: (entry: string) => Promise<string | null>;
}

/** Counts list reads, so an older answer never replaces a newer one. */
let reads = 0;

export const useRecipeLibraryStore = create<RecipeLibraryStore>((set, get) => {
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
    starters: null,
    error: null,
    selected: null,
    imported: null,
    copying: null,
    importing: false,
    importError: null,
    using: null,

    load: async () => {
      const read = ++reads;
      try {
        const [list, starters] = await Promise.all([
          commands.recipeLibraryList(),
          commands.recipeStarterList(),
        ]);
        if (read === reads) set({ list, starters, error: null });
      } catch (error) {
        if (read === reads) set({ error: errorMessage(error) });
      }
    },

    onChanged: () => {
      if (get().list) void get().load();
    },

    select: selected => set({ selected }),

    importFile: async () => {
      if (get().importing) return;
      set({ importing: true, importError: null });
      try {
        const imported = await commands.recipeImportToLibrary();
        if (imported) set({ imported: { ...imported, starter: false }, selected: imported.entry });
        await get().load();
      } catch (error) {
        set({ importError: errorMessage(error) });
      } finally {
        set({ importing: false });
      }
    },

    copyStarter: async id => {
      if (get().copying) return;
      set({ copying: id, importError: null });
      try {
        const copied = await commands.recipeLibraryCopyStarter(id);
        set({ imported: { ...copied, starter: true }, selected: copied.entry });
        await get().load();
      } catch (error) {
        set({ importError: errorMessage(error) });
      } finally {
        set({ copying: null });
      }
    },

    dismissImported: () => set({ imported: null, importError: null }),
    openUse: using => set({ using }),
    closeUse: () => set({ using: null }),

    exportRecipe: async (source, name) => {
      const notify = useAppStore.getState().notify;
      try {
        const done = await commands.recipeExport(source, name);
        if (done) notify({ status: 'done', title: `Exported ${done.file}`, detail: done.path });
      } catch (error) {
        notify({
          status: 'failed',
          title: 'Could not export the recipe',
          detail: errorMessage(error),
        });
      }
    },

    removeRevision: (entry, revision) =>
      removal(() => commands.recipeLibraryRemoveRevision(entry, revision, versionOf(entry))),

    remove: async entry => {
      const refused = await removal(() => commands.recipeLibraryRemove(entry, versionOf(entry)));
      if (!refused && get().selected === entry) set({ selected: null });
      return refused;
    },
  };
});

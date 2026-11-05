/**
 * The save to library sheet: a project's recipe saved to the recipe
 * library as the next revision of the library recipe it came from, or as
 * a new library recipe. The sheet plans first, so the person reads what
 * changes against the newest revision, and Save sends back the versions
 * the plan read: an entry or recipe that moved since is refused (`moved`),
 * read again, and the person looks before saving. A save always adds a
 * revision and never overwrites one. An agent's proposal opens the same
 * sheet from its approval card; only the sheet's Save writes, and that
 * closes the approval.
 */
import { create } from 'zustand';
import { commands, errorMessage, isErrorPayload, type RecipeSavePlan } from '@/ipc/client';
import { openRecipesTab } from './open-recipes-tab';
import { useAppStore } from './use-app-store';
import { useRecipeLibraryStore } from './use-recipe-library-store';
import { useRecipesStore } from './use-recipes-store';

export interface SaveSheet {
  project: string;
  recipe: string;
  /** The library recipe "New revision of" adds to; none when the recipe has none left. */
  revisionOf: string | null;
  /** The chosen target: that library recipe, or null for a new one. */
  entry: string | null;
  plan: RecipeSavePlan | null;
  planning: boolean;
  error: string | null;
  /** The library recipe or the recipe moved since the sheet opened, and was read again. */
  moved: boolean;
  saving: boolean;
}

interface RecipeSaveStore {
  sheet: SaveSheet | null;
  /**
   * Opens the sheet on a project's recipe. `entry` presets the target (null
   * for a new library recipe); left out, it is the library recipe the
   * recipe came from while the library still has it.
   */
  open: (project: string, recipe: string, entry?: string | null) => Promise<void>;
  /** From an agent's approval card: the project's Recipes tab, then the sheet. */
  show: (project: string, recipe: string, entry: string | null) => Promise<void>;
  setTarget: (entry: string | null) => Promise<void>;
  /** Saves as planned; true once saved. */
  save: () => Promise<boolean>;
  close: () => void;
}

const MOVED = 'moved';

/** Counts plans, so an older answer never replaces a newer one. */
let plans = 0;

export const useRecipeSaveStore = create<RecipeSaveStore>((set, get) => {
  const patch = (change: Partial<SaveSheet>) => {
    const sheet = get().sheet;
    if (sheet) set({ sheet: { ...sheet, ...change } });
  };
  const plan = async (sheet: SaveSheet) => {
    const read = ++plans;
    patch({ planning: true, error: null });
    try {
      const next = await commands.recipeLibraryPlanSave(sheet.project, sheet.recipe, sheet.entry);
      if (read === plans) patch({ plan: next, planning: false });
    } catch (error) {
      if (read === plans) patch({ plan: null, planning: false, error: errorMessage(error) });
    }
  };
  return {
    sheet: null,

    open: async (project, recipe, entry) => {
      const sheet: SaveSheet = {
        project,
        recipe,
        revisionOf: null,
        entry: null,
        plan: null,
        planning: true,
        error: null,
        moved: false,
        saving: false,
      };
      set({ sheet });
      const read = ++plans;
      try {
        const library = useRecipeLibraryStore.getState();
        const [doc] = await Promise.all([commands.recipeGet(project, recipe), library.load()]);
        if (read !== plans) return;
        const from = doc.recipe.from;
        const kept =
          from &&
          useRecipeLibraryStore
            .getState()
            .list?.some(e => e.id === from.entry && e.uid === from.uid)
            ? from.entry
            : null;
        const revisionOf = entry ?? kept;
        const ready = { ...sheet, revisionOf, entry: entry === undefined ? revisionOf : entry };
        set({ sheet: ready });
        await plan(ready);
      } catch (error) {
        if (read === plans) patch({ planning: false, error: errorMessage(error) });
      }
    },

    show: async (project, recipe, entry) => {
      if (await openRecipesTab(project)) await get().open(project, recipe, entry);
    },

    setTarget: async entry => {
      const sheet = get().sheet;
      if (!sheet || sheet.entry === entry) return;
      const next = { ...sheet, entry, plan: null, moved: false };
      set({ sheet: next });
      await plan(next);
    },

    save: async () => {
      const sheet = get().sheet;
      const planned = sheet?.plan;
      if (!sheet || !planned || planned.same || sheet.saving || sheet.planning) return false;
      patch({ saving: true, error: null });
      try {
        const saved = await commands.recipeLibrarySave({
          project: sheet.project,
          recipe: sheet.recipe,
          entry: planned.entry,
          expected: planned.entryVersion,
          recipeExpected: planned.recipeVersion,
        });
        set({ sheet: null });
        useAppStore.getState().notify({
          status: 'done',
          title: planned.entry
            ? `Saved ${planned.recipeName} as r${saved.revision} of ${planned.entryName}`
            : `Saved ${planned.recipeName} as a new library recipe`,
        });
        useRecipesStore.getState().onChanged(sheet.project);
        await useRecipeLibraryStore.getState().load();
        return true;
      } catch (error) {
        if (isErrorPayload(error) && error.code === MOVED) {
          patch({ saving: false, moved: true });
          const now = get().sheet;
          if (now) await plan(now);
        } else patch({ saving: false, error: errorMessage(error) });
        return false;
      }
    },

    close: () => {
      plans += 1;
      set({ sheet: null });
    },
  };
});

/**
 * The What changed sheet: what the library's newest revision of the
 * library recipe a project's recipe came from changes against it, and Use
 * rN, which copies that revision into the project as a new recipe. The
 * project's own recipe is never replaced.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RecipeCompare } from '@/ipc/client';
import { usedNotice } from './recipe-library';
import { useAppStore } from './use-app-store';
import { useRecipesStore } from './use-recipes-store';

export interface CompareSheet {
  project: string;
  recipe: string;
  compare: RecipeCompare | null;
  error: string | null;
  using: boolean;
}

interface RecipeCompareStore {
  sheet: CompareSheet | null;
  open: (project: string, recipe: string) => Promise<void>;
  /** Copies the newest revision into the project; true once made. */
  useLatest: () => Promise<boolean>;
  close: () => void;
}

/** Counts reads, so an older answer never replaces a newer one. */
let reads = 0;

export const useRecipeCompareStore = create<RecipeCompareStore>((set, get) => {
  const patch = (change: Partial<CompareSheet>) => {
    const sheet = get().sheet;
    if (sheet) set({ sheet: { ...sheet, ...change } });
  };
  return {
    sheet: null,

    open: async (project, recipe) => {
      const read = ++reads;
      set({ sheet: { project, recipe, compare: null, error: null, using: false } });
      try {
        const compare = await commands.recipeLibraryCompare(project, recipe);
        if (read === reads) patch({ compare });
      } catch (error) {
        if (read === reads) patch({ error: errorMessage(error) });
      }
    },

    useLatest: async () => {
      const sheet = get().sheet;
      const compare = sheet?.compare;
      if (!sheet || !compare || sheet.using) return false;
      patch({ using: true, error: null });
      try {
        const used = await commands.recipeLibraryUse({
          project: sheet.project,
          entry: compare.entry,
          revision: compare.latest,
          name: null,
        });
        set({ sheet: null });
        useAppStore.getState().notify({ status: 'done', ...usedNotice(used, compare.latest) });
        useRecipesStore.getState().onChanged(sheet.project);
        return true;
      } catch (error) {
        patch({ using: false, error: errorMessage(error) });
        return false;
      }
    },

    close: () => {
      reads += 1;
      set({ sheet: null });
    },
  };
});

/**
 * A project's animation recipes for the Motion view's Recipes tab, and the
 * base clip picker that makes one from a library clip. Nothing here bakes:
 * a recipe is a file in the project until a later update applies it.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RecipeDoc, type RecipeSummary } from '@/ipc/client';

interface RecipesStore {
  /** The project the list belongs to. */
  project: string | null;
  list: RecipeSummary[] | null;
  error: string | null;
  /** The base clip picker is open for this project. */
  picking: string | null;
  creating: boolean;
  pickError: string | null;

  load: (project: string) => Promise<void>;
  /** Reloads when the list shown is the project's. */
  onChanged: (project: string) => void;
  openPicker: (project: string) => void;
  closePicker: () => void;
  /** Makes a recipe on the library clip `clip` and closes the picker. */
  create: (clip: string, name: string) => Promise<RecipeDoc | null>;
}

export const useRecipesStore = create<RecipesStore>((set, get) => ({
  project: null,
  list: null,
  error: null,
  picking: null,
  creating: false,
  pickError: null,

  load: async project => {
    if (get().project !== project) set({ project, list: null, error: null });
    try {
      const list = await commands.recipeList(project);
      if (get().project === project) set({ list, error: null });
    } catch (error) {
      if (get().project === project) set({ error: errorMessage(error) });
    }
  },

  onChanged: project => {
    if (get().project === project) void get().load(project);
  },

  openPicker: project => set({ picking: project, pickError: null }),
  closePicker: () => set({ picking: null, pickError: null }),

  create: async (clip, name) => {
    const project = get().picking;
    if (!project || get().creating) return null;
    set({ creating: true, pickError: null });
    try {
      const doc = await commands.recipeCreate(project, clip, name.trim() || null);
      set({ creating: false, picking: null });
      await get().load(project);
      return doc;
    } catch (error) {
      set({ creating: false, pickError: errorMessage(error) });
      return null;
    }
  },
}));

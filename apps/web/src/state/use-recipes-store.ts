/**
 * A project's animation recipes for the Motion view's Recipes tab, and the
 * base clip picker that makes one from a library clip, and Import, which
 * brings a `.recipe.json` in as a new recipe. Nothing here bakes: a recipe
 * is a file in the project until a later update applies it.
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
  /** The last import into a project and what to know about it (the import warning). */
  imported: ImportNote | null;
  importing: boolean;

  load: (project: string) => Promise<void>;
  /** Reloads when the list shown is the project's. */
  onChanged: (project: string) => void;
  openPicker: (project: string) => void;
  closePicker: () => void;
  /** Makes a recipe on the library clip `clip` (the rest pose when null) and closes the picker. */
  create: (clip: string | null, name: string) => Promise<RecipeDoc | null>;
  /** Picks a `.recipe.json` with the system dialog and imports it into `project`. */
  importFile: (project: string) => Promise<void>;
  dismissImported: () => void;
}

export interface ImportNote {
  project: string;
  /** The recipe made, or null when the import was refused. */
  name: string | null;
  /** What to know: blanks that opened, the base clip, a taken id; or why it was refused. */
  notes: string[];
}

/** Counts list reads, so an older answer never replaces a newer one. */
let reads = 0;

export const useRecipesStore = create<RecipesStore>((set, get) => ({
  project: null,
  list: null,
  error: null,
  picking: null,
  creating: false,
  pickError: null,
  imported: null,
  importing: false,

  load: async project => {
    if (get().project !== project) set({ project, list: null, error: null });
    const read = ++reads;
    const latest = () => read === reads && get().project === project;
    try {
      const list = await commands.recipeList(project);
      if (latest()) set({ list, error: null });
    } catch (error) {
      if (latest()) set({ error: errorMessage(error) });
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

  importFile: async project => {
    if (get().importing) return;
    set({ importing: true });
    try {
      const used = await commands.recipeImportToProject(project);
      if (used) set({ imported: { project, name: used.doc.recipe.name, notes: used.notes } });
      await get().load(project);
    } catch (error) {
      set({ imported: { project, name: null, notes: [errorMessage(error)] } });
    } finally {
      set({ importing: false });
    }
  },

  dismissImported: () => set({ imported: null }),
}));

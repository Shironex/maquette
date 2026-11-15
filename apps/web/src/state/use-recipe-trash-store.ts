/**
 * Remove recipe: a project's recipe and its draft marks into the workspace
 * trash, with Undo on the notice that puts both back. The core refuses it
 * while the recipe bakes. Removing the recipe open in the editor goes back
 * to the list; the editor offers it only with no unsaved edits.
 */
import { create } from 'zustand';
import { commands, errorMessage } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { useRecipeEditorStore } from './use-recipe-editor-store';
import { useRecipesStore } from './use-recipes-store';

interface RecipeTrashStore {
  /** The recipe being removed or put back now, by `project/id` or `project/trash`. */
  busy: string | null;
  /** Removes the recipe; false when it was refused (the notice says why). */
  remove: (project: string, id: string, name: string) => Promise<boolean>;
  /** Puts a removed recipe back; false when it was refused. */
  restore: (project: string, trash: string, name: string) => Promise<boolean>;
}

export const useRecipeTrashStore = create<RecipeTrashStore>((set, get) => ({
  busy: null,

  remove: async (project, id, name) => {
    if (get().busy) return false;
    set({ busy: `${project}/${id}` });
    const app = useAppStore.getState();
    try {
      const done = await commands.recipeRemove(project, id);
      const editor = useRecipeEditorStore.getState();
      if (editor.editing?.project === project && editor.editing.id === id)
        await editor.leave({ to: 'list' });
      app.notify({
        status: 'done',
        title: `${done.name} is in the workspace trash`,
        detail: 'Undo puts it back with its draft marks.',
        action: { label: 'Undo', run: () => void get().restore(project, done.trash, done.name) },
      });
      return true;
    } catch (error) {
      app.notify({
        status: 'failed',
        title: `Could not remove ${name}`,
        detail: errorMessage(error),
      });
      return false;
    } finally {
      set({ busy: null });
      useRecipesStore.getState().onChanged(project);
    }
  },

  restore: async (project, trash, name) => {
    if (get().busy) return false;
    set({ busy: `${project}/${trash}` });
    const app = useAppStore.getState();
    try {
      await commands.recipeRestore(project, trash);
      app.notify({ status: 'done', title: `${name} is back`, detail: 'It is in Recipes again.' });
      return true;
    } catch (error) {
      app.notify({
        status: 'failed',
        title: `Could not restore ${name}`,
        detail: errorMessage(error),
      });
      return false;
    } finally {
      set({ busy: null });
      useRecipesStore.getState().onChanged(project);
    }
  },
}));

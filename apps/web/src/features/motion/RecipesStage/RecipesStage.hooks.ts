import { useEffect, useState } from 'react';
import type { RecipeSummary } from '@/ipc/client';
import { libraryNote } from '@/state/recipe-library';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeCompareStore } from '@/state/use-recipe-compare-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { useRecipeSaveStore } from '@/state/use-recipe-save-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/**
 * The open project's recipes, loaded when the project changes, the one
 * open in the editor and whether a bake is open in the review. A recipe with unsaved edits stays open while the
 * project changes, so the question about them can still be answered on it.
 */
export function useRecipesStage() {
  const project = useAppStore(s => s.workspace?.project?.id ?? null);
  const owner = useRecipesStore(s => s.project);
  const list = useRecipesStore(s => s.list);
  const error = useRecipesStore(s => s.error);
  const load = useRecipesStore(s => s.load);
  const openPicker = useRecipesStore(s => s.openPicker);
  const editing = useRecipeEditorStore(s => s.editing);
  const opening = useRecipeEditorStore(s => s.opening);
  const openError = useRecipeEditorStore(s => s.openError);
  const reviewed = useRecipeBakesStore(s => s.reviewing?.project ?? null);
  const library = useRecipeLibraryStore(s => s.list);
  const loadLibrary = useRecipeLibraryStore(s => s.load);
  const imported = useRecipesStore(s => s.imported);
  const importing = useRecipesStore(s => s.importing);
  useEffect(() => {
    if (project) void load(project);
  }, [project, load]);
  // The "Library has rN" notes read the recipe library.
  useEffect(() => {
    void loadLibrary();
  }, [loadLibrary]);
  const editingProject = editing?.project ?? null;
  useEffect(() => {
    if (project && editingProject && editingProject !== project)
      void useRecipeEditorStore.getState().leave({ to: 'project', project });
  }, [project, editingProject]);
  const mine = owner === project;
  return {
    project,
    recipes: mine ? list : null,
    error: mine ? error : null,
    editing: Boolean(editing),
    /** A bake of this project is open in the review. */
    reviewing: Boolean(project) && reviewed === project,
    opening,
    openError,
    retry: () => (project ? void load(project) : undefined),
    newRecipe: () => (project ? openPicker(project) : undefined),
    edit: (id: string) =>
      project ? void useRecipeEditorStore.getState().open(project, id) : undefined,
    /** The import warning: what the last import into this project should know. */
    imported: imported && imported.project === project ? imported : null,
    importing,
    importFile: () => (project ? void useRecipesStore.getState().importFile(project) : undefined),
    dismissImported: useRecipesStore.getState().dismissImported,
    noteOf: (r: RecipeSummary) => libraryNote(r.from, library),
    saveToLibrary: (id: string) =>
      project ? void useRecipeSaveStore.getState().open(project, id) : undefined,
    exportFile: (r: RecipeSummary) =>
      void useRecipeLibraryStore
        .getState()
        .exportRecipe({ project, recipe: r.id, entry: null, revision: null }, r.name),
    compare: (id: string) =>
      project ? void useRecipeCompareStore.getState().open(project, id) : undefined,
  };
}

/** Whether a recipe row's More menu is open. */
export function useRowMenu() {
  const [open, setOpen] = useState(false);
  return { open, toggle: () => setOpen(o => !o), close: () => setOpen(false) };
}

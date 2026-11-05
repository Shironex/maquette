import { useEffect } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/**
 * The open project's recipes, loaded when the project changes, and the one
 * open in the editor. A recipe with unsaved edits stays open while the
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
  useEffect(() => {
    if (project) void load(project);
  }, [project, load]);
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
    opening,
    openError,
    retry: () => (project ? void load(project) : undefined),
    newRecipe: () => (project ? openPicker(project) : undefined),
    edit: (id: string) =>
      project ? void useRecipeEditorStore.getState().open(project, id) : undefined,
  };
}

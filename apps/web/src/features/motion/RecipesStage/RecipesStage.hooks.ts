import { useEffect } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/** The open project's recipes, loaded when the project changes. */
export function useRecipesStage() {
  const project = useAppStore(s => s.workspace?.project?.id ?? null);
  const owner = useRecipesStore(s => s.project);
  const list = useRecipesStore(s => s.list);
  const error = useRecipesStore(s => s.error);
  const load = useRecipesStore(s => s.load);
  const openPicker = useRecipesStore(s => s.openPicker);
  useEffect(() => {
    if (project) void load(project);
  }, [project, load]);
  const mine = owner === project;
  return {
    recipes: mine ? list : null,
    error: mine ? error : null,
    retry: () => (project ? void load(project) : undefined),
    newRecipe: () => (project ? openPicker(project) : undefined),
  };
}

import { useState } from 'react';
import { commands, errorMessage, type RecipeUsed } from '@/ipc/client';
import { openRecipesTab } from '@/state/open-recipes-tab';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeLibraryStore, type UseTarget } from '@/state/use-recipe-library-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/** The revision the sheet copies, while it is open. */
export const useUseTarget = () => useRecipeLibraryStore(s => s.using);

/** The sheet's project (the open one first), the name, the copy and what to know about it. */
export function useUseRecipeSheet(target: UseTarget) {
  const workspace = useAppStore(s => s.workspace);
  const projects = workspace?.projects ?? [];
  const [project, setProject] = useState(workspace?.project?.id ?? projects[0]?.id ?? '');
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [used, setUsed] = useState<RecipeUsed | null>(null);
  const close = useRecipeLibraryStore.getState().closeUse;
  const use = async () => {
    if (!project || busy) return;
    setBusy(true);
    setError(null);
    try {
      const made = await commands.recipeLibraryUse({
        project,
        entry: target.entry,
        revision: target.revision,
        name: name.trim() || null,
      });
      setUsed(made);
      useRecipesStore.getState().onChanged(project);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  return {
    projects: projects.map(p => ({ value: p.id, label: p.name })),
    project,
    setProject,
    projectName: projects.find(p => p.id === project)?.name ?? project,
    name,
    setName,
    busy,
    error,
    used,
    close,
    use: () => void use(),
    openIt: () => {
      close();
      void openRecipesTab(project);
    },
  };
}

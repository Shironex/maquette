import { useEffect } from 'react';
import { isStarterId } from '@/state/recipe-starters';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';

/**
 * The recipe library and the starters, read when the section opens, and
 * the entry or starter on show: the selected one, else the first library
 * recipe, else the first starter.
 */
export function useLibraryRecipes() {
  const list = useRecipeLibraryStore(s => s.list);
  const starters = useRecipeLibraryStore(s => s.starters);
  const error = useRecipeLibraryStore(s => s.error);
  const selected = useRecipeLibraryStore(s => s.selected);
  const imported = useRecipeLibraryStore(s => s.imported);
  const importing = useRecipeLibraryStore(s => s.importing);
  const importError = useRecipeLibraryStore(s => s.importError);
  const load = useRecipeLibraryStore(s => s.load);
  const root = useAppStore(s => s.workspace?.root);
  useEffect(() => {
    void load();
  }, [load, root]);
  const { select, importFile, dismissImported } = useRecipeLibraryStore.getState();
  const starter = isStarterId(selected)
    ? (starters?.find(s => s.id === selected) ?? null)
    : list?.length
      ? null
      : (starters?.[0] ?? null);
  const summary = starter ? null : (list?.find(e => e.id === selected) ?? list?.[0] ?? null);
  return {
    list,
    starters,
    error,
    imported,
    importing,
    importError,
    summary,
    starter,
    select,
    importFile: () => void importFile(),
    dismissImported,
    retry: () => void load(),
  };
}

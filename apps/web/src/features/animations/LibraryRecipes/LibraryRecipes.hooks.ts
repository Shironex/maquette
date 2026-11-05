import { useEffect } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';

/** The recipe library, read when the section opens, and the entry on show. */
export function useLibraryRecipes() {
  const list = useRecipeLibraryStore(s => s.list);
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
  return {
    list,
    error,
    imported,
    importing,
    importError,
    summary: list?.find(e => e.id === selected) ?? list?.[0] ?? null,
    select,
    importFile: () => void importFile(),
    dismissImported,
    retry: () => void load(),
  };
}

import type { RecipeStarter } from '@/ipc/client';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';

/** The selected starter's actions: Use in project, and Save to library as an editable copy. */
export function useStarterDetail(starter: RecipeStarter) {
  const copying = useRecipeLibraryStore(s => s.copying === starter.id);
  const store = useRecipeLibraryStore.getState();
  return {
    copying,
    use: () => store.openUse({ entry: starter.id, revision: null, name: starter.name }),
    copy: () => void store.copyStarter(starter.id),
  };
}

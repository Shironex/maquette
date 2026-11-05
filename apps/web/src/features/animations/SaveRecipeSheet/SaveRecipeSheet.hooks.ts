import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { useRecipeSaveStore } from '@/state/use-recipe-save-store';

/** The target picker's value for a new library recipe, never a library recipe id. */
export const NEW_ENTRY = '(new)';

/** The open sheet, the name of the library recipe "New revision of" adds to, and the actions. */
export function useSaveRecipeSheet() {
  const sheet = useRecipeSaveStore(s => s.sheet);
  const list = useRecipeLibraryStore(s => s.list);
  const { setTarget, save, close } = useRecipeSaveStore.getState();
  const plan = sheet?.plan ?? null;
  const known = list?.find(e => e.id === sheet?.revisionOf)?.name;
  return {
    sheet,
    plan,
    revisionName: known ?? (plan?.entry ? plan.entryName : (sheet?.revisionOf ?? '')),
    target: sheet?.entry ?? NEW_ENTRY,
    setTarget: (value: string) => void setTarget(value === NEW_ENTRY ? null : value),
    save: () => void save(),
    close,
  };
}

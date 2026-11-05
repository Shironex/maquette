import { useRecipeCompareStore } from '@/state/use-recipe-compare-store';

/** The open What changed sheet and its actions. */
export function useRecipeCompareSheet() {
  const sheet = useRecipeCompareStore(s => s.sheet);
  const { useLatest, close } = useRecipeCompareStore.getState();
  return { sheet, compare: sheet?.compare ?? null, useLatest: () => void useLatest(), close };
}

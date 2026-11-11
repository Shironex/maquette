import { clock } from '@/lib/clock';
import { DRAFTS_LEGEND, draftsBanner } from '@/state/recipe-drafts';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/** The open recipe's draft marks in words, or null when a person has saved since. */
export function useRecipeDrafts() {
  const drafts = useRecipeEditorStore(s => s.editing?.doc.drafts ?? null);
  if (!drafts) return null;
  return { ...draftsBanner(drafts, clock.now().getTime()), legend: DRAFTS_LEGEND };
}

import { clock } from '@/lib/clock';
import { useRememberedFlag } from '@/lib/useRememberedFlag';
import { DRAFTS_LEGEND, draftsBanner } from '@/state/recipe-drafts';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/** Whether the drafts banner is folded to its title, remembered across recipes. */
const FOLDED_KEY = 'maquette.recipes.draftsFolded';

/** The open recipe's draft marks in words, or null when a person has saved since. */
export function useRecipeDrafts() {
  const drafts = useRecipeEditorStore(s => s.editing?.doc.drafts ?? null);
  const [folded, setFolded] = useRememberedFlag(FOLDED_KEY);
  if (!drafts) return null;
  return {
    ...draftsBanner(drafts, clock.now().getTime()),
    legend: DRAFTS_LEGEND,
    folded,
    toggle: () => setFolded(!folded),
  };
}

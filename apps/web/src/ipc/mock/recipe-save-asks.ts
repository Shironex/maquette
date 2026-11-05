/**
 * Recipe save approvals in mock mode: an agent's propose_recipe_save waits
 * as a card until the person saves the recipe in the save to library sheet
 * (the save closes it) or the card's Deny turns it down. No answer to the
 * card writes anything. `?agents=recipe-save` (with `?library=demo`) opens
 * with one waiting: Frostveil's lantern recipe as r4 of its library recipe.
 * The fake agents make them; the recipe library handlers reach the ones of
 * the backend made last.
 */
import type { Approval } from '../client';
import { clock } from '@/lib/clock';

export type RecipeSaveAsks = ReturnType<typeof createRecipeSaveAsks>;

let current: RecipeSaveAsks | null = null;

/** The approvals of the backend made last, if it has agents. */
export const currentSaveAsks = (): RecipeSaveAsks | null => current;

/** The demo's proposal: the lantern recipe as the next revision of the library recipe it came from. */
function demoAsk(): Approval {
  return {
    id: 'recipe-save-approval-1',
    kind: 'recipe-save',
    project: 'frostveil',
    runId: null,
    runNumber: null,
    step: null,
    job: 'reach-for-the-lantern',
    assetName: 'Reach for the lantern',
    service: null,
    detail:
      'An agent proposes saving Reach for the lantern to the recipe library as r4 of Reach for the lantern. 2 lines of changes.',
    items: [],
    requestedAtMs: clock.nowMs() - 30_000,
    recipeSave: { recipe: 'reach-for-the-lantern', entry: 'reach-for-the-lantern', revision: 4 },
  };
}

export function createRecipeSaveAsks(changed: () => void, demo = false) {
  let asks: Approval[] = demo ? [demoAsk()] : [];
  const made = {
    approvals: (): Approval[] => asks,
    /** The person saved `recipe` in the sheet: nothing waits any more. */
    close(project: string, recipe: string) {
      const left = asks.filter(a => a.project !== project || a.job !== recipe);
      if (left.length === asks.length) return;
      asks = left;
      changed();
    },
    /** The card's Deny: false when `id` is no recipe save approval. */
    deny(id: string): boolean {
      if (!asks.some(a => a.id === id)) return false;
      asks = asks.filter(a => a.id !== id);
      changed();
      return true;
    },
  };
  current = made;
  return made;
}

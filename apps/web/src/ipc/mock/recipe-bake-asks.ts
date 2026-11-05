/**
 * Recipe bake approvals in mock mode: an agent's add_recipe_bake waits as
 * a card until the person adds the bake in the review (approved), a new
 * bake of the recipe replaces it (cancelled), or the card's Deny turns it
 * down (the staged bake stays). `?agents=recipe` opens with one waiting on
 * Frostveil's lantern recipe, with `?library=demo`. The fake agents make
 * them; the recipe handlers reach the ones of the backend made last.
 */
import type { Approval, RecipeBakeRecord } from '../client';
import { clock } from '@/lib/clock';

export type RecipeBakeAsks = ReturnType<typeof createRecipeBakeAsks>;

let current: RecipeBakeAsks | null = null;

/** The approvals of the backend made last, if it has agents. */
export const currentAsks = (): RecipeBakeAsks | null => current;

/** `demo`: an agent asks to add the demo bake once it is read. */
export function createRecipeBakeAsks(changed: () => void, demo = false) {
  let asks: Approval[] = [];
  let next = 1;
  const find = (project: string, recipe: string) =>
    asks.find(a => a.project === project && a.job === recipe);
  const made = {
    demo,
    approvals: (): Approval[] => asks,
    /** An agent asks to add `record`'s bake; asking again keeps the one card. */
    ask(record: RecipeBakeRecord): string {
      const found = find(record.project, record.recipe);
      if (found) return found.id;
      const result = record.result;
      const from = result?.fromVersion ?? 1;
      const model = result?.assetName ?? record.asset;
      const warns = record.review?.warns ?? 0;
      const approval: Approval = {
        id: `recipe-approval-${next++}`,
        kind: 'recipe-bake',
        project: record.project,
        runId: null,
        runNumber: null,
        step: null,
        job: record.recipe,
        assetName: `${model} v${from + 1}`,
        service: null,
        detail: `${record.recipeName} baked onto ${model} v${from}${result ? ` with the new clip ${result.clip}` : ''}. ${warns === 0 ? 'Every number of the review passes.' : `${warns} ${warns === 1 ? 'number of the review warns' : 'numbers of the review warn'}.`}`,
        items: [],
        requestedAtMs: clock.nowMs() - 40_000,
        asset: record.asset,
        version: from,
      };
      asks = [...asks, approval];
      changed();
      return approval.id;
    },
    /** The bake was added, or a new bake replaced it. */
    close(project: string, recipe: string) {
      const found = find(project, recipe);
      if (!found) return;
      asks = asks.filter(a => a !== found);
      changed();
    },
    /** The card's Deny: false when `id` is no recipe bake approval. */
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

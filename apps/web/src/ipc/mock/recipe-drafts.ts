/**
 * Draft marks in mock mode, as the core keeps them (recipes/drafts.rs):
 * what agents changed in a recipe since a person saved it, dropped of the
 * changes and keys the recipe no longer has. With `?agents=drafts`, an
 * agent keyed the chest and the head of the pose keys demo a few minutes
 * ago.
 */
import type { RecipeDrafts, RecipeFile } from '../client';

/** The demo agent, as people see its name. */
export const DEMO_AGENT = 'Motion agent';

/** The pose keys demo's marks: its change and two of its keys, written `minutes` before `now`. */
export function demoDrafts(now: Date, minutes = 4): RecipeDrafts {
  return {
    by: DEMO_AGENT,
    at: new Date(now.getTime() - minutes * 60_000).toISOString(),
    writes: 2,
    made: false,
    layers: ['l-1'],
    keys: [
      { layer: 'l-1', clip: 0, frame: 20 },
      { layer: 'l-1', clip: 1, frame: 10 },
    ],
    clips: false,
    time: false,
    blanks: false,
    mirror: false,
  };
}

/** `drafts` without what `recipe` no longer has; undefined when nothing is left. */
export function prunedDrafts(
  drafts: RecipeDrafts | undefined,
  recipe: RecipeFile
): RecipeDrafts | undefined {
  if (!drafts) return undefined;
  const layers = (drafts.layers ?? []).filter(id => recipe.layers.some(l => l.id === id));
  const keys = (drafts.keys ?? []).filter(k =>
    recipe.layers.some(
      l =>
        l.id === k.layer &&
        (l.keys ?? []).some(p => (p.clip ?? 0) === k.clip && p.frame === k.frame)
    )
  );
  const left = { ...drafts, layers, keys };
  const empty =
    !left.made &&
    !layers.length &&
    !keys.length &&
    !(left.clips || left.time || left.blanks || left.mirror);
  return empty ? undefined : left;
}

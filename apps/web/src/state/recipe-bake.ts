/**
 * A recipe's bake as the Motion view shows it: whether Apply can start one,
 * its progress, and the review's numbers against their limits. Pure helpers; the store is
 * use-recipe-bakes-store.ts.
 */
import type {
  RecipeBakeRecord,
  RecipeBakeResult,
  RecipeBakeReview,
  RecipeReviewStatus,
} from '@/ipc/client';

/** A reach may end this far from its target, cm (the core's limit). */
export const GAP_LIMIT_CM = 2;

/** The store key of the bake of `recipe` in `project`. */
export const bakeKey = (project: string, recipe: string) => `${project}/${recipe}`;

/** Why Apply cannot start a bake now, or null when it can. */
export function applyBlocked(r: {
  open: number;
  layers: number;
  model: string | null;
  running: boolean;
}): string | null {
  if (r.running) return 'This recipe is baking';
  if (r.layers === 0) return 'Add a change first';
  if (r.open > 0) return `Fill the ${r.open === 1 ? 'open blank' : `${r.open} open blanks`} first`;
  if (!r.model) return 'No rigged model in this project to bake onto';
  return null;
}

/** Why a bake is out of date once its recipe changed (the core's words). */
export const RECIPE_CHANGED = 'The recipe changed since this bake. Bake it again.';

/**
 * Why a finished bake is out of date by what the view knows now: the
 * recipe's `motionVersion` (its `version` for a record from before) and the model's newest `version`, as the core works it
 * out. Either unknown, the core's own `stale` from the last read stands.
 */
export function staleOf(
  record: RecipeBakeRecord | null,
  now: {
    recipeVersion?: string | null;
    motionVersion?: string | null;
    model?: { name: string; version: number } | null;
  }
): string | null {
  const result = record?.state === 'done' ? record.result : null;
  if (!result) return null;
  // The motion version leaves out the library link; a record from before has none.
  const changed = result.motionVersion
    ? Boolean(now.motionVersion) && now.motionVersion !== result.motionVersion
    : Boolean(now.recipeVersion) && now.recipeVersion !== result.recipeVersion;
  if (changed) return RECIPE_CHANGED;
  if (now.model && now.model.version !== result.fromVersion)
    return `${now.model.name} is at v${now.model.version} now, and this bake went onto v${result.fromVersion}. Bake it again.`;
  // What the view judged just now (the recipe, the model's version) beats
  // the core's last read of the same; any other reason of the core stands.
  const judged =
    record?.stale === RECIPE_CHANGED ||
    /^.+ is at v\d+ now, and this bake went onto v\d+\./.test(record?.stale ?? '');
  const known = Boolean(now.motionVersion || now.recipeVersion) && Boolean(now.model);
  return known && judged ? null : (record?.stale ?? null);
}

/** "Solving frame 12 of 31", with how far as a share of the run. */
export function progressOf(record: RecipeBakeRecord): { label: string; share: number } {
  const p = record.progress;
  if (!p) return { label: 'Starting Blender...', share: 0 };
  return { label: p.label, share: p.total > 0 ? Math.min(p.done / p.total, 1) : 0 };
}

/** How a review line reads at a glance: its icon, colour and word. */
export const STATUS_LOOK: Record<
  RecipeReviewStatus,
  { icon: 'check-circle' | 'warning' | 'minus'; tone: string; words: string }
> = {
  pass: { icon: 'check-circle', tone: 'text-sage', words: 'Passes' },
  warn: { icon: 'warning', tone: 'text-amber', words: 'Worth a look' },
  'not-measured': { icon: 'minus', tone: 'text-fg-mute', words: 'Not measured' },
};

/** "Every number passes", "1 of 7 numbers warns", "2 of 7 numbers warn". */
export function reviewSummary(review: RecipeBakeReview): string {
  const all = review.lines.length;
  if (review.warns === 0) return all === 1 ? 'The number passes' : 'Every number passes';
  return `${review.warns} of ${all} ${all === 1 ? 'number' : 'numbers'} ${review.warns === 1 ? 'warns' : 'warn'}`;
}

/** The add's label: "Add as v3", the version after the one it was baked onto. */
export const addLabel = (result: RecipeBakeResult) => `Add as v${result.fromVersion + 1}`;

/** What an added bake says: "Added as v3 of Elsheria." */
export const addedLine = (record: RecipeBakeRecord) =>
  `Added as v${record.addedVersion ?? (record.result?.fromVersion ?? 0) + 1} of ${record.result?.assetName ?? record.asset}.`;

/** "reach_out · 31 f · onto Elsheria v1 · 2.4 s". */
export function resultSummary(result: RecipeBakeResult): string {
  return [
    result.clip,
    `${result.frames} f`,
    `onto ${result.assetName} v${result.fromVersion}`,
    `${(result.seconds ?? 0).toFixed(1)} s`,
  ].join(' · ');
}

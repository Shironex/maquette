import { useEffect } from 'react';
import { applyBlocked, bakeKey, staleOf } from '@/state/recipe-bake';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';

/**
 * The rigged model a bake goes onto: the one picked in the Motion view, else
 * the project's first rigged model, as the rough preview picks it.
 */
export function useBakeModel(): { asset: string; name: string } | null {
  const view = useMotionStore(s => s.view);
  const asset = useMotionStore(s => s.asset);
  const rigged = view?.models.filter(m => m.rig && !m.error) ?? [];
  const model = rigged.find(m => m.asset === asset) ?? rigged[0] ?? null;
  return model ? { asset: model.asset, name: model.name } : null;
}

/**
 * A recipe's bake and its Apply: the last bake (read once), whether Apply
 * can start one and why not, and Cancel.
 */
export function useRecipeBake(
  project: string,
  recipe: string,
  counts: { open: number; layers: number },
  /** The recipe's version as the view read it, to tell a bake out of date. */
  version?: string | null
) {
  const key = bakeKey(project, recipe);
  const record = useRecipeBakesStore(s => s.bakes[key]);
  const busy = useRecipeBakesStore(s => Boolean(s.busy[key]));
  const error = useRecipeBakesStore(s => s.errors[key] ?? null);
  const model = useBakeModel();
  const baked = useMotionStore(s => s.view?.models.find(m => m.asset === record?.asset));
  useEffect(() => {
    if (useRecipeBakesStore.getState().bakes[key] === undefined)
      void useRecipeBakesStore.getState().load(project, recipe);
  }, [key, project, recipe]);
  const running = record?.state === 'running';
  const store = useRecipeBakesStore.getState();
  return {
    record: record ?? null,
    /** The baked model's name as the Motion view lists it. */
    assetName: baked?.name ?? record?.asset ?? null,
    /** Why the last bake is out of date, or null. */
    stale: staleOf(record ?? null, { recipeVersion: version, model: baked ?? null }),
    busy,
    error,
    model,
    running,
    blocked: applyBlocked({ ...counts, model: model?.asset ?? null, running }),
    /** Starts the bake; `expected` is the recipe's version as read. */
    apply: (expected?: string | null) =>
      model ? store.start(project, recipe, model.asset, expected) : Promise.resolve(false),
    cancel: () => void store.cancel(project, recipe),
    /** Opens the finished bake in the review. */
    review: () => store.openReview(project, recipe),
  };
}

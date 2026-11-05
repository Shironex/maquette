/**
 * Recipe bakes: each recipe's last bake, started by Apply or by an agent.
 * The core runs one headless Blender run per recipe and stages its model in
 * the workspace cache; the records stay up to date through the recipe bake
 * changed event, whichever view is on show.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RecipeBakeRecord } from '@/ipc/client';
import { bakeKey } from './recipe-bake';

interface RecipeBakesStore {
  /** Bakes by `project/recipe`; null once read and never baked. */
  bakes: Record<string, RecipeBakeRecord | null>;
  /** The keys whose start or cancel is on its way. */
  busy: Record<string, boolean>;
  /** What a start or cancel was refused with, by key. */
  errors: Record<string, string>;
  /** Reads a recipe's last bake. */
  load: (project: string, recipe: string) => Promise<void>;
  /** Starts a bake onto `asset`; `expected` is the recipe's version as read. */
  start: (
    project: string,
    recipe: string,
    asset: string,
    expected?: string | null
  ) => Promise<boolean>;
  cancel: (project: string, recipe: string) => Promise<void>;
  onChanged: (record: RecipeBakeRecord) => void;
}

const without = <T>(map: Record<string, T>, key: string) => {
  const next = { ...map };
  delete next[key];
  return next;
};

export const useRecipeBakesStore = create<RecipeBakesStore>((set, get) => {
  const keep = (record: RecipeBakeRecord) => {
    const key = bakeKey(record.project, record.recipe);
    const known = get().bakes[key];
    // An event can overtake a slower answer: the newer record wins.
    if (known && (known.updatedMs ?? 0) > (record.updatedMs ?? 0)) return;
    set({ bakes: { ...get().bakes, [key]: record } });
  };
  const busy = (key: string, on: boolean) =>
    set({ busy: on ? { ...get().busy, [key]: true } : without(get().busy, key) });
  return {
    bakes: {},
    busy: {},
    errors: {},

    load: async (project, recipe) => {
      const key = bakeKey(project, recipe);
      try {
        const record = await commands.recipeBakeGet(project, recipe);
        if (record) keep(record);
        else if (!get().bakes[key]) set({ bakes: { ...get().bakes, [key]: null } });
      } catch (error) {
        set({ errors: { ...get().errors, [key]: errorMessage(error) } });
      }
    },

    start: async (project, recipe, asset, expected = null) => {
      const key = bakeKey(project, recipe);
      busy(key, true);
      set({ errors: without(get().errors, key) });
      try {
        keep(await commands.recipeBakeStart({ project, recipe, asset, expected }));
        return true;
      } catch (error) {
        set({ errors: { ...get().errors, [key]: errorMessage(error) } });
        return false;
      } finally {
        busy(key, false);
      }
    },

    cancel: async (project, recipe) => {
      const key = bakeKey(project, recipe);
      busy(key, true);
      try {
        await commands.recipeBakeCancel(project, recipe);
        // The event says when it stopped; reading it now shows a stop that was quick.
        await get().load(project, recipe);
      } catch (error) {
        set({ errors: { ...get().errors, [key]: errorMessage(error) } });
      } finally {
        busy(key, false);
      }
    },

    onChanged: keep,
  };
});

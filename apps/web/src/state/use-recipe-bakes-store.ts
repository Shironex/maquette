/**
 * Recipe bakes: each recipe's last bake, started by Apply or by an agent.
 * The core runs one headless Blender run per recipe and stages its model in
 * the workspace cache; the records stay up to date through the recipe bake
 * changed event, whichever view is on show. A finished bake opens in the
 * review, where Add makes it the model's next version.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RecipeBakeRecord } from '@/ipc/client';
import { addedLine, bakeKey } from './recipe-bake';
import { useAppStore } from './use-app-store';
import { useMotionStore } from './use-motion-store';
import { useProjectsStore } from './use-projects-store';

/** The bake open in the review. */
export interface ReviewedBake {
  project: string;
  recipe: string;
}

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
  /** The bake open in the review, or none. */
  reviewing: ReviewedBake | null;
  /** What an add was refused with, by key. */
  addErrors: Record<string, string>;
  openReview: (project: string, recipe: string) => void;
  closeReview: () => void;
  /**
   * Opens the review from elsewhere (an agent's approval card): the
   * project, the Motion view and its Recipes tab.
   */
  showReview: (project: string, recipe: string) => Promise<void>;
  /** Adds the reviewed bake as its model's next version. */
  add: (project: string, recipe: string) => Promise<boolean>;
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
    reviewing: null,
    addErrors: {},

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

    openReview: (project, recipe) => {
      set({
        reviewing: { project, recipe },
        addErrors: without(get().addErrors, bakeKey(project, recipe)),
      });
      void get().load(project, recipe);
    },

    closeReview: () => set({ reviewing: null }),

    showReview: async (project, recipe) => {
      const app = useAppStore.getState();
      if (app.workspace?.project?.id !== project) {
        const switched = await useProjectsStore.getState().switchTo(project);
        if (!switched) return;
      }
      useMotionStore.getState().setTab('recipes');
      get().openReview(project, recipe);
      useAppStore.getState().setView('motion');
    },

    add: async (project, recipe) => {
      const key = bakeKey(project, recipe);
      busy(key, true);
      set({ addErrors: without(get().addErrors, key) });
      try {
        const record = await commands.recipeBakeAdd(project, recipe);
        keep(record);
        set({ reviewing: null });
        useAppStore.getState().notify({
          status: 'done',
          title: addedLine(record),
          detail: record.result?.clipNote ?? undefined,
        });
        void useMotionStore.getState().onChanged(project);
        return true;
      } catch (error) {
        set({ addErrors: { ...get().addErrors, [key]: errorMessage(error) } });
        return false;
      } finally {
        busy(key, false);
      }
    },
  };
});

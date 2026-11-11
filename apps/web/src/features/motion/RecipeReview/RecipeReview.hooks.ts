import { useEffect, useState } from 'react';
import type { RecipeReviewFix } from '@/ipc/client';
import { bakeKey, staleOf } from '@/state/recipe-bake';
import { applyFix } from '@/state/apply-fix';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeBakesStore } from '@/state/use-recipe-bakes-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';

/** Why Try it laid nothing on the recipe. */
export const FIX_GONE =
  'The recipe no longer has what this fix changes. Bake it again for fresh fixes.';
export const FIX_WAITS = 'Answer the question about your unsaved edits first, then try it again.';

/** What the review shows: the bake, the base clip, or both side by side. */
export type ReviewShow = 'both' | 'baked' | 'base';

/** Frames per second a clip's length falls back to when the library has none. */
const FPS = 30;

/**
 * The bake open in the review: its record, the model it went onto as it is
 * now and the base clip from the library (for the side by side), one
 * playhead for both, whether it is out of date, an agent's approval waiting
 * on it, and Add, Bake again and Close.
 */
export function useRecipeReview() {
  const reviewing = useRecipeBakesStore(s => s.reviewing);
  const project = reviewing?.project ?? '';
  const recipe = reviewing?.recipe ?? '';
  const key = bakeKey(project, recipe);
  const record = useRecipeBakesStore(s => s.bakes[key] ?? null);
  const busy = useRecipeBakesStore(s => Boolean(s.busy[key]));
  const addError = useRecipeBakesStore(s => s.addErrors[key] ?? null);
  const summary = useRecipesStore(s =>
    s.project === project ? (s.list?.find(r => r.id === recipe) ?? null) : null
  );
  const result = record?.result ?? null;
  const model = useMotionStore(s => s.view?.models.find(m => m.asset === record?.asset) ?? null);
  const library = useAnimationsStore(s => s.view);
  const approval = useAgentsStore(
    s =>
      s.status?.approvals.find(
        a => a.kind === 'recipe-bake' && a.project === project && a.job === recipe
      ) ?? null
  );
  const [show, setShow] = useState<ReviewShow>('both');
  const [share, setShare] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [fixError, setFixError] = useState<string | null>(null);
  useEffect(() => {
    if (!useAnimationsStore.getState().view) void useAnimationsStore.getState().load();
  }, []);
  // A bake on the rest pose names no base clip.
  const rest = result ? !result.baseClip : false;
  const item = rest
    ? null
    : (library?.items.find(
        i => i.clip.id === result?.baseClip || i.clip.name === result?.baseClip
      ) ?? null);
  const frames = result?.frames ?? 0;
  const length = item?.clip.length || frames / FPS || 1;
  const stale = staleOf(record, {
    recipeVersion: summary?.version || null,
    motionVersion: summary?.motionVersion || null,
    model: model ? { name: model.name, version: model.version } : null,
  });
  const store = useRecipeBakesStore.getState();
  return {
    project,
    record,
    result,
    review: record?.review ?? null,
    stale,
    busy,
    addError,
    approval,
    model: model ? { asset: model.asset, name: model.name, file: model.file } : null,
    clip: item ? { id: item.clip.id, name: item.clip.name, file: item.file } : null,
    rest,
    show,
    setShow,
    /** One playhead for both pictures, a share of the clip, looping while it plays. */
    play: {
      share,
      // Frames count as the library does: keys 0 to `frames`.
      frame: Math.round(share * frames),
      frames,
      playing,
      toggle: () => setPlaying(p => !p),
      tick: (delta: number) => {
        if (playing) setShare(s => (s + delta / length) % 1);
      },
    },
    sheet: {
      frames: result?.sheet ?? [],
      /** Stops the player on frame `frame`, as a pose sheet tile picks it. */
      seek: (frame: number) => {
        setPlaying(false);
        setShare(frames > 0 ? Math.min(frame / frames, 1) : 0);
      },
    },
    fixes: {
      list: record?.review?.fixes ?? [],
      error: fixError,
      /**
       * Opens the recipe in the editor with `fix` laid on its draft, unsaved,
       * and leaves the review: the person saves and bakes again.
       */
      tryFix: async (fix: RecipeReviewFix) => {
        setFixError(null);
        const editor = useRecipeEditorStore.getState();
        await editor.open(project, recipe);
        const editing = useRecipeEditorStore.getState().editing;
        if (editing?.project !== project || editing.id !== recipe) {
          setFixError(FIX_WAITS);
          return;
        }
        const next = applyFix(editing.draft, fix.edit);
        if (!next) {
          setFixError(FIX_GONE);
          return;
        }
        editor.edit(() => next);
        store.closeReview();
      },
    },
    close: store.closeReview,
    add: () => void store.add(project, recipe, record?.finishedMs ?? null),
    /** Bakes the recipe again onto the same model; the list shows it run. */
    bakeAgain: async () => {
      if (!record) return;
      const started = await store.start(project, recipe, record.asset, summary?.version || null);
      if (started) store.closeReview();
    },
  };
}

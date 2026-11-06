import { useMemo } from 'react';
import type { RecipeFile } from '@/ipc/client';
import { recipeOfDraft } from './recipe-clips';
import { frameOf, shareOf } from './recipe-draft';
import { frameOfShare, plain, timeline, type Timeline } from './recipe-timeline';
import { useRecipeEditorStore } from './use-recipe-editor-store';

export interface DraftTimeline {
  /** The recipe the draft makes. */
  recipe: RecipeFile;
  timeline: Timeline;
  /** It plays its one clip whole, as round 1 did: shares are frames over the clip's count. */
  plain: boolean;
  /** The bake frame where a change at `share` stands. */
  toFrame: (share: number) => number;
  /** Where a change stands when it is put on bake frame `frame`. */
  toShare: (frame: number) => number;
}

/** The timeline of `recipe`, with shares and frames turned into each other as its changes read them. */
export function draftTimeline(recipe: RecipeFile): DraftTimeline {
  const t = timeline(recipe);
  const isPlain = plain(recipe);
  const frames = recipe.base.frames;
  return {
    recipe,
    timeline: t,
    plain: isPlain,
    toFrame: share => (isPlain ? frameOf(share, frames) : frameOfShare(t, share)),
    toShare: frame => {
      if (isPlain) return shareOf(frame, frames);
      const at = Math.min(Math.max(Math.round(frame), 0), t.frames);
      return t.keys[at]?.share ?? 0;
    },
  };
}

/** The open recipe's draft as a timeline, worked out again only when the draft changes. */
export function useRecipeTimeline(): DraftTimeline | null {
  const editing = useRecipeEditorStore(s => s.editing);
  const file = editing?.doc.recipe;
  const draft = editing?.draft;
  return useMemo(
    () => (file && draft ? draftTimeline(recipeOfDraft(file, draft)) : null),
    [file, draft]
  );
}

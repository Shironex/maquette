import { useMemo } from 'react';
import type { RecipeFile } from '@/ipc/client';
import { draftTimeline } from '@/state/use-recipe-timeline';

/** A revision's timeline, so its changes read as frames of the whole joined recipe. */
export const useHoldsTimeline = (recipe: RecipeFile) =>
  useMemo(() => draftTimeline(recipe), [recipe]);

import { useMemo } from 'react';
import type { RecipeLayer } from '@/ipc/client';
import type { StageGrips } from '@/features/viewer/RecipePreview';
import { gripSpans } from '@/state/recipe-grips';
import type { DraftTimeline } from '@/state/use-recipe-timeline';

/**
 * The draft's grips on the bake's frames for the preview to lay with the
 * other changes, as the bake lays them; null for a recipe with none.
 */
export function usePreviewGrips(
  dt: DraftTimeline | null,
  layers: readonly RecipeLayer[] | undefined
): StageGrips | null {
  return useMemo(() => {
    if (!dt || !layers?.some(l => l.kind === 'grip')) return null;
    return {
      spans: gripSpans(dt.timeline, layers),
      frames: dt.timeline.frames,
      frameOf: dt.toFrame,
    };
  }, [dt, layers]);
}

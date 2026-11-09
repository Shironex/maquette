import { useEffect, useMemo } from 'react';
import type { RecipeLayer } from '@/ipc/client';
import type { StagePins } from '@/features/viewer/recipe-pins';
import { pinSpans } from '@/state/recipe-pins';
import type { DraftTimeline } from '@/state/use-recipe-timeline';
import { useRecipePinChecks } from '@/state/use-recipe-pin-checks';

/**
 * The draft's pins on the bake's frames for the preview to hold, with its
 * checks sent where each pin's settings read them; cleared when the preview
 * goes.
 */
export function usePreviewPins(
  dt: DraftTimeline | null,
  layers: readonly RecipeLayer[] | undefined
): StagePins | null {
  useEffect(() => () => useRecipePinChecks.getState().set([]), []);
  return useMemo(() => {
    if (!dt || !layers?.some(l => l.kind === 'pin')) return null;
    return {
      spans: pinSpans(dt.timeline, layers),
      frames: dt.timeline.frames,
      onChecks: useRecipePinChecks.getState().set,
    };
  }, [dt, layers]);
}

/**
 * The mock bake's grips: each grip change of the recipe on the bake's
 * frames with the bones the model takes (the core reads them from the rig,
 * see bake/grips.rs), and the review's line per grip. The stand-in models
 * have every finger, so a mock bake poses every bone.
 */
import type { RecipeBakeResult, RecipeDoc, RecipeGripResult, RecipeReviewLine } from '@/ipc/client';
import { spanOn } from '@/state/recipe-grips';
import { timeline } from '@/state/recipe-timeline';

/** Each grip as a fake bake lays it: every bone of its pose on the model. */
export function gripResultsOf(doc: RecipeDoc): RecipeGripResult[] {
  const t = timeline(doc.recipe);
  return doc.recipe.layers
    .filter(l => l.kind === 'grip')
    .map(layer => {
      const span = spanOn(t, layer);
      const bones = Object.keys(layer.pose ?? {}).length;
      return {
        layer: layer.id,
        bone: layer.bone,
        entry: layer.from?.entry ?? null,
        revision: layer.from?.revision ?? null,
        frames: span ? [span.first, span.last] : null,
        bones,
        posed: bones,
        missing: [],
      };
    });
}

/** The review's line per grip, as the core words it. */
export function gripLines(result: RecipeBakeResult): RecipeReviewLine[] {
  return (result.grips ?? []).map(grip => {
    const span = grip.frames
      ? `Frames ${grip.frames[0]} to ${grip.frames[1]}.`
      : 'No frame falls within its range, so it poses nothing.';
    const whole = grip.missing.length === 0 && grip.frames !== null;
    const left =
      grip.missing.length === 0
        ? ''
        : ` ${result.assetName} has no bone for ${grip.missing.slice(0, 4).join(', ')}, so the grip leaves ${grip.missing.length === 1 ? 'it' : 'them'}.`;
    return {
      id: `grip-${grip.layer}`,
      label: 'Grip',
      about: `${grip.bone} (${grip.layer})`,
      value: `${grip.posed} of ${grip.bones} bones`,
      limit: 'every bone',
      measured: null,
      max: null,
      unit: null,
      status: whole ? 'pass' : 'warn',
      note: `${span}${left}`,
    };
  });
}

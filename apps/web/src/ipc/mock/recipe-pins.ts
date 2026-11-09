/**
 * The mock bake's pins: each pin of the recipe on the bake's frames with a
 * drift the review can show (the core measures it in Blender, see
 * bake/pins.rs), and the review's drift line held to 1 cm.
 */
import type { RecipeBakeResult, RecipeDoc, RecipePinResult, RecipeReviewLine } from '@/ipc/client';
import { PIN_DRIFT_LIMIT_CM, heldFrames, pinSpans } from '@/state/recipe-pins';
import { timeline } from '@/state/recipe-timeline';

/** Each pin as a fake bake measures it: a foot holds within its limit, a hand drifts a little more. */
export function pinResultsOf(doc: RecipeDoc): RecipePinResult[] {
  return pinSpans(timeline(doc.recipe), doc.recipe.layers).map((pin, i) => {
    const held = heldFrames(pin);
    const drift = pin.bone.endsWith('Foot') ? 0.4 + i * 0.1 : 0.7;
    return {
      layer: pin.layer,
      bone: pin.bone,
      first: pin.first,
      last: pin.last,
      driftCm: held ? drift : null,
      frame: held ? Math.round((held[0] + held[1]) / 2) : null,
      baseDriftCm: held ? 6.8 : null,
    };
  });
}

/** The review's line per pin, as the core words it. */
export function pinLines(result: RecipeBakeResult): RecipeReviewLine[] {
  return (result.pins ?? []).map(pin => {
    const span = `Frames ${pin.first} to ${pin.last}.`;
    const without =
      pin.baseDriftCm == null ? '' : ` Without the pin it moves ${pin.baseDriftCm.toFixed(1)} cm.`;
    const drift = pin.driftCm;
    const warn = drift != null && drift > PIN_DRIFT_LIMIT_CM;
    return {
      id: `pin-${pin.layer}`,
      label: 'Pin drift',
      about: `${pin.bone} (${pin.layer})`,
      value: drift == null ? 'not measured' : `${drift.toFixed(1)} cm`,
      limit: `${PIN_DRIFT_LIMIT_CM} cm`,
      measured: drift,
      max: PIN_DRIFT_LIMIT_CM,
      unit: 'cm',
      status: drift == null ? 'not-measured' : warn ? 'warn' : 'pass',
      note:
        drift == null
          ? `${span} Its eases leave no frame where it holds whole.`
          : warn && pin.frame != null
            ? `${span} It leaves its spot most at frame ${pin.frame}.${without}`
            : `${span}${without}`,
    };
  });
}

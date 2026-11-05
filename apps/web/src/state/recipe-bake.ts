/**
 * A recipe's bake as the Motion view shows it: whether Apply can start one,
 * and the result's numbers in short lines. Pure helpers; the store is
 * use-recipe-bakes-store.ts.
 */
import type { RecipeBakeRecord, RecipeBakeResult } from '@/ipc/client';

/** What the result says comes after a bake. */
export const REVIEW_NEXT = 'Review and add as a version comes next';

/** A reach may end this far from its target, cm (the core's limit). */
export const GAP_LIMIT_CM = 2;

/** The store key of the bake of `recipe` in `project`. */
export const bakeKey = (project: string, recipe: string) => `${project}/${recipe}`;

/** Why Apply cannot start a bake now, or null when it can. */
export function applyBlocked(r: {
  open: number;
  layers: number;
  model: string | null;
  running: boolean;
}): string | null {
  if (r.running) return 'This recipe is baking';
  if (r.layers === 0) return 'Add a change first';
  if (r.open > 0) return `Fill the ${r.open === 1 ? 'open blank' : `${r.open} open blanks`} first`;
  if (!r.model) return 'No rigged model in this project to bake onto';
  return null;
}

/** "Solving frame 12 of 31", with how far as a share of the run. */
export function progressOf(record: RecipeBakeRecord): { label: string; share: number } {
  const p = record.progress;
  if (!p) return { label: 'Starting Blender...', share: 0 };
  return { label: p.label, share: p.total > 0 ? Math.min(p.done / p.total, 1) : 0 };
}

const cm = (n: number) => `${n.toFixed(1)} cm`;
const mm = (n: number) => `${n.toFixed(1)} mm`;

/** One measured number of a finished bake, with whether it needs a look. */
export interface BakeLine {
  id: string;
  label: string;
  value: string;
  note: string | null;
  bad: boolean;
}

/** The result's numbers: each reach's gap and elbow or knee, the feet, the rig. */
export function resultLines(result: RecipeBakeResult): BakeLine[] {
  const slide = result.slideCm ?? 0;
  const baseSlide = result.baseSlideCm ?? 0;
  const reaches: BakeLine[] = result.reaches.map(r => ({
    id: `reach-${r.layer}`,
    label: `Reach ${r.bone}`,
    value: r.gapCm == null ? 'not measured' : `${cm(r.gapCm)} from its target`,
    note: r.flipFrame == null ? null : `The joint bends the other way from frame ${r.flipFrame}.`,
    bad: !r.reached || r.flipFrame != null,
  }));
  const feet: BakeLine = {
    id: 'feet',
    label: 'Foot slide',
    value: cm(slide),
    note: `The base clip slides ${cm(baseSlide)} on this model.`,
    bad: slide > baseSlide + 0.5 && result.slideFrames != null,
  };
  const stretch: BakeLine =
    result.stretchMm == null
      ? {
          id: 'stretch',
          label: 'Rig stretch',
          value: 'not measured',
          note: result.stretchNote,
          bad: false,
        }
      : {
          id: 'stretch',
          label: 'Rig stretch',
          value: mm(result.stretchMm),
          note: result.baseStretchMm == null ? null : `The base clip: ${mm(result.baseStretchMm)}.`,
          bad: result.baseStretchMm != null && result.stretchMm > result.baseStretchMm + 5,
        };
  return [...reaches, feet, stretch];
}

/** "reach_out · 31 f · onto Elsheria v1 · 2.4 s". */
export function resultSummary(result: RecipeBakeResult): string {
  return [
    result.clip,
    `${result.frames} f`,
    `onto ${result.assetName} v${result.fromVersion}`,
    `${(result.seconds ?? 0).toFixed(1)} s`,
  ].join(' · ');
}

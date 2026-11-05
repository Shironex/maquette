/**
 * The rough preview's live checks in words: at a reach's end, a hand or foot
 * left over 2 cm from its target, or an elbow or knee bent the other way
 * than the clip has it. The viewer measures (features/viewer/recipe-checks);
 * slice 4's review measures the real bake.
 */
import type { RecipeLayer } from '@/ipc/client';
import { boneWords } from './recipe-bones';

/** A gap over this many metres at a reach's end is worth a look. */
export const GAP_LIMIT = 0.02;

export interface ReachCheck {
  layer: string;
  /** Metres from the hand or foot to the target at the reach's end. */
  gap: number;
  /** The middle joint bends the other way than the clip has it. */
  flipped: boolean;
}

/** A line per check worth a look, naming the change. */
export function checkLines(
  checks: readonly ReachCheck[],
  layers: readonly RecipeLayer[]
): string[] {
  return checks.flatMap(c => {
    const layer = layers.find(l => l.id === c.layer);
    if (!layer) return [];
    const who = `${boneWords(layer.bone)} (${c.layer})`;
    const joint = /Foot$/.test(layer.bone) ? 'knee' : 'elbow';
    const lines: string[] = [];
    if (c.gap > GAP_LIMIT)
      lines.push(
        `${who} stops ${Math.round(c.gap * 100)} cm short of its target at the reach's end.`
      );
    if (c.flipped) lines.push(`${who}: the ${joint} bends the other way than the clip has it.`);
    return lines;
  });
}

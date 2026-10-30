/**
 * The fingers in a normalised rest pose: how far a finger may curl and
 * still count as straight, and how far each hand was straightened.
 */
import type { NormalizeReport } from '@/ipc/client';

/** A finger may curl this far in all and count as straight, as the readiness reads it. */
export const FINGER_SLACK_DEG = 15;

/**
 * "fingers straightened", or how far each hand went when its mesh held only
 * part of the way: "fingers straightened (left hand 25%, right hand left as
 * it is)".
 */
export function fingersSummary(report: NormalizeReport): string {
  if (report.fingers === 'keep') return 'fingers kept';
  const sides = Object.entries(report.straightened ?? {});
  if (sides.every(([, s]) => (s.share ?? 0) >= 1)) return 'fingers straightened';
  const words = sides.map(([side, s]) => {
    const share = s.share ?? 0;
    return `${side} hand ${share > 0 ? `${Math.round(share * 100)}%` : 'left as it is'}`;
  });
  return `fingers straightened (${words.join(', ')})`;
}

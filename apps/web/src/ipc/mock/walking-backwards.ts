/**
 * `?inbox=walk`: a rigged character whose walk clip carries it backwards,
 * its hips travelling against the way it faces, beside the usual models.
 * Its checks say so as the core's walk row does.
 */
import type { InboxItem } from '../client';

/** The character, made from Elsheria's row: it faces +X and walks along -X. */
export function walkingBackwards(base: InboxItem, dir: string, now: number): InboxItem {
  const report = base.report && {
    ...base.report,
    animations: ['Walk', 'Idle'],
    clipTravel: [
      { clip: 'Walk', node: 'mixamorig:Hips', along: [-1.24, 0.02] as [number, number] },
    ],
  };
  const checks = base.checks.map(check =>
    check.id === 'animations'
      ? { ...check, status: 'pass' as const, title: '2 animations', detail: 'Walk, Idle' }
      : check
  );
  return {
    ...base,
    id: 'ash_walker.glb',
    name: 'Ash walker',
    file: `${dir}/ash_walker.glb`,
    modifiedMs: now,
    report,
    checks: [
      ...checks,
      {
        id: 'walk-facing',
        status: 'warning',
        title: 'Walk travels against the face',
        detail:
          'Walk moves 1.24 m along -X while the model faces +X: it walks backwards. Turn the model to face its walk, or keep the clip as a backward walk.',
        fixes: [],
      },
    ],
  };
}

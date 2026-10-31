/** Words for the rig doctor: its six steps and how a case stands. */
import type { Check, RigRecord, RigStep } from '@/ipc/client';

/** The Inbox checks that offer the rig doctor when they are not a pass. */
export const RIG_CHECKS = new Set(['rig', 'finger-weights', 'arm-weights']);

export function offersRigDoctor(check: Check): boolean {
  return RIG_CHECKS.has(check.id) && check.status !== 'pass';
}

export const isRunning = (r: RigRecord) => r.state === 'testing' || r.state === 'fixing';

const ORDER: RigStep[] = ['read', 'accept', 'before', 'fix', 'verify', 'land'];

/** The next version the draft would become. */
export const nextVersion = (r: RigRecord) => (r.source.version ?? 1) + 1;

export interface StepLine {
  step: RigStep;
  title: string;
  detail: string;
  /** done, on (the current one) or todo. */
  mark: 'done' | 'on' | 'todo';
}

function cells(r: RigRecord): number {
  return r.before?.rows.length ?? 0;
}

/** The six steps of the left column, as this case stands. */
export function stepLines(r: RigRecord): StepLine[] {
  const n = nextVersion(r);
  const v = n - 1;
  const at = ORDER.indexOf(r.step);
  const count = r.defects.length;
  const rows: Record<RigStep, [string, string]> = {
    read: ['Read the model', 'Rig check and the Inbox checks'],
    accept: [`Accept as v${v}`, 'The original stays as it came'],
    before: [
      'Pose test before',
      r.before
        ? `${cells(r)} cells, ${count} defect${count === 1 ? '' : 's'} found`
        : isRunning(r)
          ? 'Rendering on a copy'
          : 'Pose sheets, front and side',
    ],
    fix: [
      'Fix',
      r.model.oneIsland ? 'Narrow reweight, one island' : 'Per island weights, smooth seams',
    ],
    verify: [
      'Verify',
      r.checks.length && r.checks.every(c => c.ok)
        ? 'All checks pass'
        : 'Only joints and weights change',
    ],
    land: [
      `Land as v${n}`,
      r.landed !== null
        ? `Added as v${r.landed}`
        : at === 5
          ? `Waits for Add as v${n}`
          : 'Through the Inbox checks',
    ],
  };
  const landed = r.landed !== null;
  return ORDER.map((step, i) => ({
    step,
    title: rows[step][0],
    detail: rows[step][1],
    mark: landed || i < at ? 'done' : i === at ? 'on' : 'todo',
  }));
}

/** "Rig doctor: pose test 9 of 13", for the Inbox's way back to a case. */
export function caseLine(r: RigRecord): string {
  if (r.progress)
    return `${r.name}: ${r.progress.label.toLowerCase()} ${r.progress.done} of ${r.progress.total}`;
  if (r.state === 'fixed') return `${r.name}: draft v${nextVersion(r)} waits`;
  if (r.state === 'stopped') return `${r.name}: stopped`;
  if (r.state === 'failed') return `${r.name}: failed`;
  if (isRunning(r)) return `${r.name}: running`;
  return `${r.name}: open`;
}

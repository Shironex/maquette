/**
 * The rig doctor in mock mode: `?rig=` opens a case per board and stop; any
 * other model opens read, and walks through its pose test and fix on a
 * timer. Nothing is written; only the rig doctor view adds a draft, as in
 * the app, and the agents tools have no way to.
 */
import type { Approval, RigRecord, RigRun, RigTarget } from '../client';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import { creatureCase, oneIslandCase, refusedCase, runningCase, stoppedCase } from './rig-cases';
import { fixedCase, readCase, testedCase } from './rig-fixtures';

interface RigContext {
  state: string | null;
  pace: number;
  emit: (record: RigRecord) => void;
}

const RIG_APPROVAL = 'approval-rig-1';

const SEEDS: Record<string, () => RigRecord> = {
  read: () => testedCase('duchess-vesperine-v1', 'Duchess Vesperine'),
  compare: () => fixedCase('duchess-vesperine-v1', 'Duchess Vesperine'),
  creature: creatureCase,
  refused: refusedCase,
  stopped: stoppedCase,
  'one-island': oneIslandCase,
  running: runningCase,
};

const slug = (text: string) => text.toLowerCase().replace(/[^a-z0-9]+/g, '-');

export function createRigHandlers(state: string | null, pace: number, emit: RigContext['emit']) {
  const ctx: RigContext = { state, pace, emit };
  const cases = new Map<string, RigRecord>();
  let denied = false;
  const seed = ctx.state ? SEEDS[ctx.state]?.() : undefined;
  if (seed) cases.set(seed.id, seed);

  const put = (record: RigRecord): RigRecord => {
    cases.set(record.id, record);
    ctx.emit(structuredClone(record));
    return structuredClone(record);
  };
  const get = (id: string): RigRecord => {
    const found = cases.get(id);
    return found ?? fail('invalid', `No rig doctor case ${id}.`);
  };
  const later = (work: () => void) => setTimeout(work, ctx.pace * 4);

  const run = (id: string, what: RigRun) => {
    const c = get(id);
    if (what === 'pose-test') {
      later(() => put({ ...testedCase(id, c.name), source: c.source, input: c.input }));
      return put({
        ...c,
        state: 'testing',
        step: 'before',
        progress: { label: 'Pose test before', done: 3, total: 27 },
      });
    }
    later(() => put({ ...fixedCase(id, c.name), source: c.source, round: c.round + 1 }));
    return put({ ...c, state: 'fixing', step: 'fix', round: c.round });
  };

  return {
    /** What an agent's case waits for: an approval per case it started. */
    approvals: (): Approval[] =>
      ctx.state === 'creature' && !denied
        ? [
            {
              id: RIG_APPROVAL,
              kind: 'rig-doctor',
              project: 'frostveil',
              runId: null,
              runNumber: null,
              step: null,
              job: 'spiked-hound-v1',
              assetName: 'Spiked hound v2',
              service: null,
              detail: 'The rig doctor fixed 2 defects. Only joints and weights changed.',
              items: [],
              requestedAtMs: clock.nowMs(),
              estimate: null,
              conceptJobs: [],
              estimateUsd: null,
              asset: 'spiked-hound',
              version: 1,
              sheets: {
                before: '~/Maquette/cache/rig/spiked-hound-v1/before/sheet.svg',
                after: '~/Maquette/cache/rig/spiked-hound-v1/after/sheet.svg',
              },
            },
          ]
        : [],
    deny: (id: string) => {
      if (id === RIG_APPROVAL) denied = true;
    },
    handlers: {
      rig_list: () => [...cases.values()].map(c => structuredClone(c)),
      rig_get: (args: Args) => structuredClone(get(args.id as string)),
      rig_open: (args: Args) => {
        const target = args.target as RigTarget;
        const id = target.inbox
          ? `${slug(target.inbox)}-inbox`
          : `${target.asset}-v${target.version ?? 1}`;
        const found = cases.get(id);
        if (found) return structuredClone(found);
        const name = target.inbox ?? target.asset ?? 'Model';
        const record = readCase(id, name, target.inbox);
        if (!target.inbox) {
          record.source = {
            inbox: null,
            project: target.project,
            asset: target.asset,
            version: target.version ?? 1,
          };
        }
        return put(record);
      },
      rig_accept: (args: Args) => {
        const c = get(args.id as string);
        return put({
          ...c,
          step: 'before',
          source: {
            inbox: null,
            project: args.project as string,
            asset: (args.asset as string | null) ?? slug(c.name),
            version: 1,
          },
        });
      },
      rig_run: (args: Args) => run(args.id as string, args.what as RigRun),
      rig_cancel: (args: Args) => {
        put({ ...get(args.id as string), state: 'cancelled', progress: null });
        return null;
      },
      rig_decide_joints: (args: Args) => {
        const c = get(args.id as string);
        const move = c.jointMove;
        if (!move) fail('invalid', 'No joint move waits for an answer.');
        return put({
          ...c,
          jointMove: { ...move, state: args.approve ? 'approved' : 'declined' },
          draft: args.approve ? move.draft : c.draft,
        });
      },
      rig_land: (args: Args) => {
        const c = get(args.id as string);
        return put({ ...c, state: 'landed', landed: (c.source.version ?? 1) + 1 });
      },
      rig_discard: (args: Args) =>
        put({ ...get(args.id as string), state: 'discarded', draft: null, jointMove: null }),
      rig_save_sheet: (args: Args) => `~/Maquette/exports/rig/${args.id as string}_sheet.png`,
      rig_open_blender: () => null,
    },
  };
}

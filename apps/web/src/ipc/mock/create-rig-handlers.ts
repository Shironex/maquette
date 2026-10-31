/**
 * The rig doctor in mock mode: `?rig=` opens a case per board and stop; any
 * other model opens read, and walks through its pose test and fix on a
 * timer. Nothing is written; only the rig doctor view adds a draft, as in
 * the app, and the agents tools have no way to.
 */
import type { Approval, RigChainEdit, RigRecord, RigRoute, RigRun, RigTarget } from '../client';
import { chainChanged } from '@/state/rig';
import { clock } from '@/lib/clock';
import { fail, type Args } from './backend-options';
import {
  creatureCase,
  creatureFixedCase,
  oneIslandCase,
  refusedCase,
  runningCase,
  stoppedCase,
} from './rig-cases';
import { registerMockFile } from '../files';
import { figure, fixedCase, readCase, testedCase } from './rig-fixtures';

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
  'creature-fixed': creatureFixedCase,
  refused: refusedCase,
  stopped: stoppedCase,
  'one-island': oneIslandCase,
  running: runningCase,
};

/** The table a first pose test reads on the creature route. */
const CREATURE_READ = () => creatureCase().creature;

/** Drops what the runs made, as the core does when the sheets go stale. */
const startOver = (c: RigRecord): RigRecord => ({
  ...c,
  state: 'ready',
  step: c.source.version === null ? 'accept' : 'before',
  before: null,
  after: null,
  defects: [],
  stats: [],
  fixSteps: [],
  checks: [],
  jointMove: null,
  round: 0,
  draft: null,
  progress: null,
  stages: [],
  log: [],
  stop: null,
  walkProof: null,
  error: null,
});

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
      later(() =>
        put({
          ...testedCase(id, c.name),
          source: c.source,
          input: c.input,
          route: c.route,
          creature: c.route === 'creature' ? (c.creature ?? CREATURE_READ()) : null,
          chainsConfirmed: c.chainsConfirmed,
          chainEdits: c.chainEdits,
          startedBy: c.startedBy,
        })
      );
      return put({
        ...c,
        state: 'testing',
        step: 'before',
        progress: { label: 'Pose test before', done: 3, total: 27 },
      });
    }
    if (what === 'walk-proof') {
      later(() => put({ ...get(id), state: 'fixed', walkProof: `${c.dir}/walk_proof.svg` }));
      registerMockFile(`${c.dir}/walk_proof.svg`, figure('walk', 'side', true));
      return put({ ...c, state: 'fixing' });
    }
    later(() =>
      put({
        ...fixedCase(id, c.name),
        source: c.source,
        round: c.round + 1,
        route: c.route,
        creature: c.creature,
        chainsConfirmed: c.chainsConfirmed,
        chainEdits: c.chainEdits,
        startedBy: c.startedBy,
      })
    );
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
      rig_set_route: (args: Args) => {
        const c = get(args.id as string);
        const route = args.route as RigRoute;
        if (c.route === route) return structuredClone(c);
        if (c.draft) fail('invalid', 'Discard the draft first: it came from the other route.');
        if (route === 'humanoid' && !c.model.humanoidOk)
          fail('invalid', 'The humanoid route needs Mixamo bone names.');
        return put({
          ...startOver(c),
          route,
          creature: null,
          chainsConfirmed: false,
          chainEdits: [],
        });
      },
      rig_confirm_chains: (args: Args) => {
        const c = get(args.id as string);
        if (!c.creature) fail('invalid', 'No chains were read yet.');
        const edits = args.edits as RigChainEdit[];
        const changed = edits.some(e => chainChanged(e, c.creature!.chains));
        const chains = c.creature.chains.map(row => {
          const edit = edits.find(e => e.key === row.key);
          if (!edit) return row;
          return {
            ...row,
            readAs: edit.side ? 'chain' : (edit.readAs ?? row.readAs),
            side: edit.side ? `${edit.side}, by hand` : row.side,
            note: null,
          };
        });
        const creature = { ...c.creature, chains };
        const kept = (c.chainEdits ?? []).filter(e => !edits.some(n => n.key === e.key));
        return changed
          ? put({
              ...startOver(c),
              creature,
              chainsConfirmed: true,
              chainEdits: [...kept, ...edits],
            })
          : put({ ...c, creature, chainsConfirmed: true });
      },
      rig_land: (args: Args) => {
        const c = get(args.id as string);
        if (c.route === 'creature' && !c.chainsConfirmed)
          fail('invalid', 'Look at the chains first: they were only read from the bone tree.');
        return put({ ...c, state: 'landed', landed: (c.source.version ?? 1) + 1 });
      },
      rig_discard: (args: Args) =>
        put({ ...get(args.id as string), state: 'discarded', draft: null, jointMove: null }),
      rig_save_sheet: (args: Args) => `~/Maquette/exports/rig/${args.id as string}_sheet.png`,
      rig_open_blender: () => null,
    },
  };
}

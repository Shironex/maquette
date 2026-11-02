/**
 * The Board's Next step strip as a pure function of what the computer and the
 * project say. Same order as get_started's next list (mcp/tools/start/next.rs):
 * the most blocking first, one sentence and one button.
 */
import type { Asset, ProjectTarget, RunState } from '@/ipc/client';
import type { BlenderTrouble } from './toolchain';

export type NextAction =
  'toolchain' | 'inbox' | 'lab' | 'import' | 'send' | 'refs' | 'pipeline' | 'repo';

export interface NextStep {
  /** Which step this is. */
  id: string;
  /** Which instance of it: a dismissal holds only while this stays the same. */
  key: string;
  /** How much waits, for a step that comes back only when there is more. */
  count?: number;
  sentence: string;
  button: string;
  action: NextAction;
  /** The assets a `send` or `refs` opens with. */
  assets?: readonly string[];
}

export interface NextFacts {
  /** Set only once a finished check says so. */
  blender: BlenderTrouble | null;
  inbox: number;
  running: boolean;
  /** The empty Board shows its first run card instead. */
  tipsShowing: boolean;
  target: ProjectTarget;
  hasRepo: boolean;
  /** An imported folder keeps its own scene and scripts, so it builds with no assets. */
  imported: boolean;
  assets: readonly Asset[];
  /** Assets a send is on its way for. */
  sending: ReadonlySet<string>;
  /** The newest run; undefined before there is one or before it is read. */
  lastRun: { id: string; state: RunState } | undefined;
}

/** What a person dismissed: a step's `key`, and its `count` then. */
export interface Dismissal {
  key: string;
  count?: number;
}

const plural = (n: number, one: string, many: string) => (n === 1 ? one : many);

/** Keyed by id alone: one instance only. */
const single = (step: Omit<NextStep, 'key'>): NextStep => ({ ...step, key: step.id });

/** A card's picture or tagged refs say it has refs; a bare card only has a stage. */
const hasRefs = (asset: Asset) =>
  (asset.refs?.length ?? 0) > 0 || Boolean(asset.thumbnail?.startsWith('refs/'));

function emptyStep({ target, tipsShowing, imported }: NextFacts): NextStep | null {
  if (tipsShowing) return null;
  if (imported) {
    return single({
      id: 'empty-imported',
      sentence: 'This folder builds from its own scene and scripts. Run the pipeline when ready.',
      button: 'Open Pipeline',
      action: 'pipeline',
    });
  }
  return target === 'game'
    ? single({
        id: 'empty',
        sentence: 'This game has no models yet. Import the ones its repo already has.',
        button: 'Import models',
        action: 'import',
      })
    : single({
        id: 'empty',
        sentence: 'This project is empty. Start with a concept in the Lab.',
        button: 'Open Concept lab',
        action: 'lab',
      });
}

function buildStep({ lastRun, hasRepo, target }: NextFacts): NextStep | null {
  if (!lastRun) {
    return single({
      id: 'run',
      sentence: 'Models are ready. Run the pipeline to build them.',
      button: 'Open Pipeline',
      action: 'pipeline',
    });
  }
  if (lastRun.state === 'failed') {
    return {
      id: 'run-failed',
      key: `run-failed:${lastRun.id}`,
      sentence: 'The last run failed. Its log says where.',
      button: 'Open Pipeline',
      action: 'pipeline',
    };
  }
  if (lastRun.state === 'cancelled') {
    return {
      id: 'run-cancelled',
      key: `run-cancelled:${lastRun.id}`,
      sentence: 'The last run was cancelled. Run the pipeline again when you are ready.',
      button: 'Open Pipeline',
      action: 'pipeline',
    };
  }
  if (lastRun.state === 'done' && !hasRepo) {
    const repo = target === 'game' ? 'game' : 'site';
    return single({
      id: 'repo',
      sentence: `The build is done and nothing shipped. Pick the ${repo} repo so Ship has somewhere to write.`,
      button: 'Open Settings',
      action: 'repo',
    });
  }
  return null;
}

function refsSteps({ assets, sending }: NextFacts): NextStep[] {
  const waiting = assets.filter(
    a => a.stage === 'refs' && a.versions.length === 0 && !sending.has(a.id)
  );
  const ready = waiting.filter(hasRefs);
  const bare = waiting.filter(a => !hasRefs(a));
  const steps: NextStep[] = [];
  if (ready.length > 0) {
    const n = ready.length;
    const ids = ready.map(a => a.id);
    steps.push({
      id: 'send',
      key: `send:${[...ids].sort().join(',')}`,
      sentence: `${n === 1 ? ready[0]!.name : `${n} assets`} ${plural(n, 'has', 'have')} refs and no model. Send to 3D; By hand spends nothing.`,
      button: 'Send to 3D',
      action: 'send',
      assets: ids,
    });
  }
  if (bare.length > 0) {
    const n = bare.length;
    const ids = bare.map(a => a.id);
    steps.push({
      id: 'refs-empty',
      key: `refs-empty:${[...ids].sort().join(',')}`,
      sentence: `${n === 1 ? bare[0]!.name : `${n} assets`} ${plural(n, 'is', 'are')} at Refs and I see no images yet. Add some before a send.`,
      button: 'Add refs',
      action: 'refs',
      assets: ids,
    });
  }
  return steps;
}

function conceptStep({ assets }: NextFacts): NextStep | null {
  const cards = assets.filter(a => a.stage === 'concept');
  if (cards.length === 0) return null;
  if (cards.some(a => a.thumbnail)) {
    return single({
      id: 'concept',
      sentence: 'Pick the concepts you like in the Lab and send them to Refs.',
      button: 'Open Concept lab',
      action: 'lab',
    });
  }
  const n = cards.length;
  return single({
    id: 'concept-empty',
    sentence: `${n === 1 ? cards[0]!.name : `${n} cards`} ${plural(n, 'has', 'have')} no picture yet. Make or pick concepts in the Lab, or drop images on the Concept column.`,
    button: 'Open Concept lab',
    action: 'lab',
  });
}

function projectSteps(facts: NextFacts): NextStep[] {
  const steps: (NextStep | null)[] = [];
  if (facts.inbox > 0) {
    const n = facts.inbox;
    steps.push({
      id: 'inbox',
      key: 'inbox',
      count: n,
      sentence: `${n} ${plural(n, 'model waits', 'models wait')} in the Inbox. Check and accept the good ones.`,
      button: 'Open Inbox',
      action: 'inbox',
    });
  }
  if (facts.assets.length === 0) steps.push(emptyStep(facts));
  else {
    steps.push(...refsSteps(facts));
    const built = facts.assets.some(a => a.versions.length > 0);
    steps.push(built ? buildStep(facts) : conceptStep(facts));
  }
  return steps.filter(step => step !== null);
}

/** Everything waiting on the person, the most blocking first. */
export function nextSteps(facts: NextFacts): NextStep[] {
  const toolchain = { button: 'Open Toolchain', action: 'toolchain' } as const;
  const steps: NextStep[] = [];
  if (facts.blender === 'missing' || facts.blender === 'broken') {
    const broken = facts.blender === 'broken';
    steps.push(
      single({
        ...toolchain,
        id: broken ? 'blender-broken' : 'blender',
        sentence: `Blender ${broken ? 'did not run' : 'is missing'}, and every build runs in it.`,
      })
    );
  }
  if (facts.running) return steps;
  steps.push(...projectSteps(facts));
  // Builds still work on the other copy, so this waits behind everything else.
  if (facts.blender === 'stale-path') {
    steps.push(
      single({
        ...toolchain,
        id: 'blender-stale',
        sentence: 'Your saved Blender path is gone, so I am using another copy.',
      })
    );
  }
  return steps;
}

/** What to do now, or null when nothing is waiting on the person. */
export function nextStep(facts: NextFacts): NextStep | null {
  return nextSteps(facts)[0] ?? null;
}

const covers = (dismissal: Dismissal, step: NextStep) =>
  dismissal.key === step.key &&
  (step.count === undefined || dismissal.count === undefined || step.count <= dismissal.count);

/** The first step not dismissed: a dismissed one never hides another behind it. */
export function visibleStep(
  steps: readonly NextStep[],
  dismissed: readonly Dismissal[]
): NextStep | null {
  return steps.find(step => !dismissed.some(d => covers(d, step))) ?? null;
}

/**
 * The dismissals that still hold: a step gone or changed is forgotten, so it
 * shows when it comes back, and a smaller count is the new mark. The same
 * array when nothing changed.
 */
export function forgetDismissed(
  dismissed: readonly Dismissal[],
  steps: readonly NextStep[]
): readonly Dismissal[] {
  const kept = dismissed.flatMap(d => {
    const step = steps.find(s => s.key === d.key);
    if (!step) return [];
    return step.count !== undefined && d.count !== undefined && step.count < d.count
      ? [{ ...d, count: step.count }]
      : [d];
  });
  const same = kept.length === dismissed.length && kept.every((d, i) => d === dismissed[i]);
  return same ? dismissed : kept;
}

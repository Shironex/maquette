/**
 * The Board's Next step strip as a pure function of what the computer and the
 * project say. Same order as get_started's next list (mcp/tools/start/next.rs):
 * the most blocking first, one sentence and one button.
 */
import type { Asset, ProjectTarget, RunState } from '@/ipc/client';
import type { BlenderTrouble } from './toolchain';

export type NextAction = 'toolchain' | 'inbox' | 'lab' | 'import' | 'send' | 'pipeline' | 'repo';

export interface NextStep {
  /** Which step this is: a step dismissed once stays away until another one shows. */
  id: string;
  sentence: string;
  button: string;
  action: NextAction;
  /** The assets a `send` opens with. */
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
  assets: readonly Asset[];
  /** Assets a send is on its way for. */
  sending: ReadonlySet<string>;
  /** The newest run; undefined before there is one or before it is read. */
  lastRun: { state: RunState } | undefined;
}

const plural = (n: number, one: string, many: string) => (n === 1 ? one : many);

function emptyStep({ target, tipsShowing }: NextFacts): NextStep | null {
  if (tipsShowing) return null;
  return target === 'game'
    ? {
        id: 'empty',
        sentence: 'This game has no models yet. Import the ones its repo already has.',
        button: 'Import models',
        action: 'import',
      }
    : {
        id: 'empty',
        sentence: 'This project is empty. Start with a concept in the Lab.',
        button: 'Open Concept lab',
        action: 'lab',
      };
}

function buildStep({ lastRun, hasRepo, target }: NextFacts): NextStep | null {
  if (!lastRun) {
    return {
      id: 'run',
      sentence: 'Models are ready. Run the pipeline to build them.',
      button: 'Open Pipeline',
      action: 'pipeline',
    };
  }
  if (lastRun.state === 'failed') {
    return {
      id: 'run-failed',
      sentence: 'The last run failed. Its log says where.',
      button: 'Open Pipeline',
      action: 'pipeline',
    };
  }
  if (lastRun.state === 'cancelled') {
    return {
      id: 'run-cancelled',
      sentence: 'The last run was cancelled. Run the pipeline again when you are ready.',
      button: 'Open Pipeline',
      action: 'pipeline',
    };
  }
  if (!hasRepo) {
    const repo = target === 'game' ? 'game' : 'site';
    return {
      id: 'repo',
      sentence: `The build is done and nothing shipped. Pick the ${repo} repo so Ship has somewhere to write.`,
      button: 'Open Settings',
      action: 'repo',
    };
  }
  return null;
}

function projectStep(facts: NextFacts): NextStep | null {
  if (facts.inbox > 0) {
    const n = facts.inbox;
    return {
      id: 'inbox',
      sentence: `${n} ${plural(n, 'model waits', 'models wait')} in the Inbox. Check and accept the good ones.`,
      button: 'Open Inbox',
      action: 'inbox',
    };
  }
  const { assets, sending } = facts;
  if (assets.length === 0) return emptyStep(facts);
  const atRefs = assets.filter(
    a => a.stage === 'refs' && a.versions.length === 0 && !sending.has(a.id)
  );
  if (atRefs.length > 0) {
    const n = atRefs.length;
    return {
      id: 'send',
      sentence: `${n === 1 ? atRefs[0]!.name : `${n} assets`} ${plural(n, 'has', 'have')} refs and no model. Send to 3D; By hand spends nothing.`,
      button: 'Send to 3D',
      action: 'send',
      assets: atRefs.map(a => a.id),
    };
  }
  if (assets.some(a => a.versions.length > 0)) return buildStep(facts);
  if (assets.some(a => a.stage === 'concept')) {
    return {
      id: 'concept',
      sentence: 'Pick the concepts you like in the Lab and send them to Refs.',
      button: 'Open Concept lab',
      action: 'lab',
    };
  }
  return null;
}

/** What to do now, or null when nothing is waiting on the person. */
export function nextStep(facts: NextFacts): NextStep | null {
  const toolchain = { button: 'Open Toolchain', action: 'toolchain' } as const;
  if (facts.blender === 'missing' || facts.blender === 'broken') {
    const broken = facts.blender === 'broken';
    return {
      ...toolchain,
      id: broken ? 'blender-broken' : 'blender',
      sentence: `Blender ${broken ? 'did not run' : 'is missing'}, and every build runs in it.`,
    };
  }
  if (facts.running) return null;
  const step = projectStep(facts);
  if (step || !facts.blender) return step;
  // Builds still work on the other copy, so this waits behind everything else.
  return {
    ...toolchain,
    id: 'blender-stale',
    sentence: 'Your saved Blender path is gone, so I am using another copy.',
  };
}

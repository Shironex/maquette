/**
 * The app update as the UI tracks it: a small state machine fed by the
 * shell's `UpdaterUpdate` events and the answers of the `updater_*`
 * commands. Both carry the same steps, so applying one twice is harmless.
 */
import type { UpdateInfo, UpdaterEvent, UpdaterMode, UpdaterStatus } from '@/ipc/client';

export type UpdaterPhase =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'current' }
  | { kind: 'pending' }
  | { kind: 'available'; update: UpdateInfo }
  | { kind: 'downloading'; update: UpdateInfo; downloaded: number; total: number | null }
  | { kind: 'ready'; update: UpdateInfo }
  | { kind: 'installing'; update: UpdateInfo }
  | { kind: 'failed'; message: string; update: UpdateInfo | null };

export interface UpdaterState {
  /** Null until the shell said whether this build updates itself. */
  mode: UpdaterMode | null;
  phase: UpdaterPhase;
}

export const INITIAL_UPDATER: UpdaterState = { mode: null, phase: { kind: 'idle' } };

/** The update a phase is about, if any. */
export function phaseUpdate(phase: UpdaterPhase): UpdateInfo | null {
  return 'update' in phase ? phase.update : null;
}

/** The starting state from the shell's status. */
export function fromStatus(status: UpdaterStatus): UpdaterState {
  const phase: UpdaterPhase = status.available
    ? status.ready
      ? { kind: 'ready', update: status.available }
      : { kind: 'available', update: status.available }
    : { kind: 'idle' };
  return { mode: status.mode, phase };
}

/** A check never pulls the UI back from an update it is already acting on. */
const BUSY = new Set<UpdaterPhase['kind']>(['downloading', 'ready', 'installing']);

/** Applies one step. */
export function reduce(state: UpdaterState, event: UpdaterEvent): UpdaterState {
  const { phase } = state;
  const next = (p: UpdaterPhase): UpdaterState => ({ ...state, phase: p });
  switch (event.type) {
    case 'checking':
      return BUSY.has(phase.kind) ? state : next({ kind: 'checking' });
    case 'available':
      return BUSY.has(phase.kind) ? state : next({ kind: 'available', update: event.update });
    case 'current':
      return BUSY.has(phase.kind) ? state : next({ kind: 'current' });
    case 'pending':
      return BUSY.has(phase.kind) ? state : next({ kind: 'pending' });
    case 'progress': {
      const update = phaseUpdate(phase);
      if (!update || phase.kind === 'ready' || phase.kind === 'installing') return state;
      return next({
        kind: 'downloading',
        update,
        downloaded: event.downloaded ?? 0,
        total: event.total,
      });
    }
    case 'ready':
      return next({ kind: 'ready', update: event.update });
    case 'failed':
      return next({ kind: 'failed', message: event.message, update: phaseUpdate(phase) });
  }
}

/** Marks the install as started, from a ready update. */
export function installing(state: UpdaterState): UpdaterState {
  return state.phase.kind === 'ready'
    ? { ...state, phase: { kind: 'installing', update: state.phase.update } }
    : state;
}

/** From 0 to 100, or undefined while the size is unknown. */
export function downloadPercent(phase: UpdaterPhase): number | undefined {
  if (phase.kind !== 'downloading' || !phase.total) return undefined;
  return Math.min(100, (phase.downloaded / phase.total) * 100);
}

/** What the About section says about updates when the updater is off. */
export const OFF_REASON: Record<Exclude<UpdaterMode, 'on'>, string> = {
  unreleased:
    'Updates arrive with the first public release. Until then, install a new version by hand.',
  'dev-build': 'This is a development build, so it does not update itself.',
  'turned-off': 'Updates are turned off for this run (MAQUETTE_NO_UPDATER).',
};

/** One line for the current phase, read out to screen readers as it changes. */
export function phaseLine(state: UpdaterState, version: string): string {
  if (state.mode === null) return '';
  if (state.mode !== 'on') return OFF_REASON[state.mode];
  const { phase } = state;
  switch (phase.kind) {
    case 'idle':
      return `Maquette ${version}. Updates are checked at launch and every few hours.`;
    case 'checking':
      return 'Checking for updates...';
    case 'current':
      return `Maquette ${version} is the newest version.`;
    case 'pending':
      return 'No release is out for this computer yet.';
    case 'available':
      return `Maquette ${phase.update.version} is out.`;
    case 'downloading':
      return `Downloading Maquette ${phase.update.version}...`;
    case 'ready':
      return `Maquette ${phase.update.version} is ready. Restart to update.`;
    case 'installing':
      return `Installing Maquette ${phase.update.version}...`;
    case 'failed':
      return phase.message;
  }
}

/** Whether the header shows its update pill. */
export function showsPill(state: UpdaterState): boolean {
  return state.mode === 'on' && phaseUpdate(state.phase) !== null && state.phase.kind !== 'failed';
}

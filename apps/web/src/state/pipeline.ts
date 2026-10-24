/**
 * Pure pipeline logic for the UI: folding runner events into the view,
 * node states for the graph, and the short summaries the status bar and
 * the top bar show. No IPC here, so all of it is unit tested.
 */
import type { NodeState, StageState } from '@maquette/ui';
import type {
  Asset,
  LogEntry,
  PipelineEvent,
  PipelineView,
  RunRecord,
  Stage,
  StarterStatus,
  StepDef,
} from '@/ipc/client';

/** Lines kept in memory for the live log. */
export const MAX_LOG_LINES = 4000;

const FINISHED = new Set(['done', 'failed', 'cancelled']);

export function isFinished(run: RunRecord): boolean {
  return FINISHED.has(run.state);
}

/** Folds one runner event into the view. Events for other projects are ignored. */
export function applyEvent(view: PipelineView, event: PipelineEvent): PipelineView {
  if (event.type === 'log') {
    const latest = view.runs[0];
    if (!latest || event.entry.runId !== latest.id) return view;
    const log = [...view.log, event.entry];
    return { ...view, log: log.length > MAX_LOG_LINES ? log.slice(-MAX_LOG_LINES) : log };
  }
  const run = event.run;
  if (run.project !== view.project) return view;
  if (run.state === 'queued') {
    if (view.queued.some(r => r.id === run.id) || view.active === run.id) return view;
    // A run that starts right away arrives as queued first, then running.
    return { ...view, queued: [...view.queued, run] };
  }
  const queued = view.queued.filter(r => r.id !== run.id);
  const known = view.runs.some(r => r.id === run.id);
  const runs = known ? view.runs.map(r => (r.id === run.id ? run : r)) : [run, ...view.runs];
  // A new run starts a fresh log.
  const log = known ? view.log : [];
  const active = isFinished(run) ? (view.active === run.id ? null : view.active) : run.id;
  return { ...view, runs, queued, active, log };
}

export interface NodeView {
  state: NodeState;
  /** From 0 to 100 while running, when the step reports how far it is. */
  progress?: number;
  label?: string;
  duration?: string;
  detail?: string;
}

/** "0.4s", "6.1s", "1m 12s". */
export function formatDuration(ms: number | null | undefined): string | undefined {
  if (ms === null || ms === undefined) return undefined;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)}s`;
  const m = Math.floor(s / 60);
  return `${m}m ${Math.round(s - m * 60)}s`;
}

/**
 * What each step's node shows. The newest run decides; a step that run
 * skipped falls back to the run before it ("failed in 24").
 */
export function nodeViews(view: PipelineView): Record<string, NodeView> {
  const out: Record<string, NodeView> = {};
  const [latest, ...older] = view.runs;
  for (const step of view.def.steps) {
    const record = latest?.steps.find(s => s.id === step.id);
    if (!record) {
      out[step.id] = { state: 'waiting' };
      continue;
    }
    switch (record.state) {
      case 'running': {
        const progress =
          typeof record.progress === 'number' ? Math.round(record.progress * 100) : undefined;
        out[step.id] =
          progress === undefined
            ? { state: 'running', label: 'Running' }
            : { state: 'running', label: 'Running', progress, duration: `${progress}%` };
        break;
      }
      case 'cached':
        out[step.id] = { state: 'cached', duration: formatDuration(record.durationMs) };
        break;
      case 'done':
        out[step.id] = { state: 'done', duration: formatDuration(record.durationMs) };
        break;
      case 'failed':
        out[step.id] = {
          state: 'failed',
          duration: formatDuration(record.durationMs),
          detail: record.message ?? undefined,
        };
        break;
      case 'cancelled':
        out[step.id] = { state: 'waiting', label: 'Cancelled' };
        break;
      case 'waiting': {
        const last = lastTime(older, step.id);
        out[step.id] = { state: 'waiting', duration: last ? `last ${last}` : undefined };
        break;
      }
      case 'skipped': {
        const before = older
          .map(r => ({ run: r, rec: r.steps.find(s => s.id === step.id) }))
          .find(x => x.rec && x.rec.state !== 'skipped');
        if (before?.rec?.state === 'failed') {
          out[step.id] = { state: 'waiting', label: `failed in ${before.run.number}` };
        } else if (before?.rec && (before.rec.state === 'done' || before.rec.state === 'cached')) {
          out[step.id] = {
            state: 'cached',
            label: 'Skipped',
            duration: formatDuration(before.rec.durationMs),
          };
        } else {
          out[step.id] = { state: 'waiting', label: 'Skipped' };
        }
        break;
      }
    }
  }
  return out;
}

function lastTime(runs: RunRecord[], step: string): string | undefined {
  for (const run of runs) {
    const rec = run.steps.find(s => s.id === step);
    if (rec && (rec.state === 'done' || rec.state === 'failed') && rec.durationMs) {
      return formatDuration(rec.durationMs);
    }
  }
  return undefined;
}

/** "2 cached · 1 running · 3 waiting". */
export function runSummary(run: RunRecord): string {
  const counts = new Map<string, number>();
  for (const s of run.steps) {
    if (s.state === 'skipped') continue;
    counts.set(s.state, (counts.get(s.state) ?? 0) + 1);
  }
  const order = ['done', 'cached', 'running', 'waiting', 'failed', 'cancelled'];
  return order
    .filter(k => counts.has(k))
    .map(k => `${counts.get(k)} ${k}`)
    .join(' · ');
}

/** "Run 25 · step 3 of 6" while running, "Run 24 failed at Capture posters" after. */
export function runStatusLine(run: RunRecord | undefined): string {
  if (!run) return 'No runs yet';
  const planned = run.steps.filter(s => s.state !== 'skipped');
  if (run.state === 'running') {
    const at = planned.findIndex(s => s.state === 'running');
    return `Run ${run.number} · step ${Math.max(1, at + 1)} of ${planned.length}`;
  }
  if (run.state === 'queued') return `Run ${run.number} queued`;
  if (run.state === 'failed') {
    const failed = run.steps.find(s => s.state === 'failed');
    return failed ? `Run ${run.number} failed at ${failed.name}` : `Run ${run.number} failed`;
  }
  if (run.state === 'cancelled') return `Run ${run.number} cancelled`;
  return `Run ${run.number} done`;
}

/** The step a failed run stopped at, for Re-run from here. */
export function failedStep(run: RunRecord | undefined): string | null {
  return run?.steps.find(s => s.state === 'failed')?.id ?? null;
}

export interface LogRow {
  t: string;
  text: string;
  level: LogEntry['level'];
  step: string | null;
}

function clock(ms: number | null): string {
  if (!ms) return '';
  const d = new Date(ms);
  return [d.getHours(), d.getMinutes(), d.getSeconds()]
    .map(n => String(n).padStart(2, '0'))
    .join(':');
}

/** Log entries as console rows, optionally for one step. */
export function logRows(log: readonly LogEntry[], step: string | 'all'): LogRow[] {
  return log
    .filter(e => step === 'all' || e.step === step || e.step === null)
    .map(e => ({ t: clock(e.atMs), text: e.text, level: e.level, step: e.step }));
}

/** The script or output a node shows under its name. */
export function stepCaption(step: StepDef): string {
  if (step.script) return step.script.split('/').pop() ?? step.script;
  if (step.compress) return `${step.compress.from} to ${step.compress.to}`;
  return step.command ?? step.kind;
}

export interface StageView {
  id: Stage;
  label: string;
  state: StageState;
}

const STAGE_ORDER: readonly Stage[] = ['concept', 'refs', 'tripo', 'blender', 'web', 'ship'];
const STAGE_LABEL: Record<Stage, string> = {
  concept: 'Concept',
  refs: 'Refs',
  tripo: 'Tripo',
  blender: 'Blender',
  web: 'Web',
  ship: 'Ship',
};

/**
 * The top bar stepper: stages before the furthest asset are done, that one
 * is current, and while a run is going (or after it failed) the Web stage
 * shows it.
 */
export function stageViews(assets: readonly Asset[], run: RunRecord | undefined): StageView[] {
  const furthest = assets.reduce((max, a) => Math.max(max, STAGE_ORDER.indexOf(a.stage)), 0);
  return STAGE_ORDER.map((id, i) => {
    let state: StageState = i < furthest ? 'done' : i === furthest ? 'current' : 'todo';
    if (id === 'web' && run && run.state === 'running') state = 'running';
    if (
      id === 'web' &&
      run &&
      run.state === 'failed' &&
      furthest >= STAGE_ORDER.indexOf('blender')
    ) {
      state = 'failed';
    }
    return { id, label: STAGE_LABEL[id], state };
  });
}

export const STAGES = STAGE_ORDER;
export { STAGE_LABEL };

/** The bytes the shipped models add up to, or the raw ones before a compress. */
export function estimatedTransfer(view: PipelineView): number {
  return view.models.reduce((sum, m) => sum + (m.shippedBytes ?? m.rawBytes ?? 0), 0);
}

/** `tools/bake_light.py` is `bake_light.py`. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** "bake_light.py, build_scene.py and render_stops.py" */
export function listNames(paths: readonly string[]): string {
  const names = paths.map(fileName);
  if (names.length <= 1) return names[0] ?? '';
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}

export function starterToast(n: number): string {
  return n === 1 ? 'Starter script updated' : `${n} starter scripts updated`;
}

/** What the Pipeline view offers for the project's starter scripts. */
export function starterOffer(starters: readonly StarterStatus[]): {
  /** Untouched old copies and missing ones: updated in one click. */
  update: string[];
  /** Edited by hand: only replaced one at a time, when asked. */
  edited: string[];
} {
  return {
    update: starters.filter(s => s.state === 'outdated' || s.state === 'missing').map(s => s.path),
    edited: starters.filter(s => s.state === 'edited').map(s => s.path),
  };
}

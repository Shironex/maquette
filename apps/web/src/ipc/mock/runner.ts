/**
 * A fake pipeline runner for mock mode. It walks the steps like the real
 * one: cached steps report a cache hit, Blender steps stream realistic
 * lines, Capture posters fails once (a missing camera) and passes after.
 */
import type { LogEntry, PipelineDef, PipelineEvent, RunRecord, StepRecord } from '../client';
import { blenderLines } from './pipeline-fixtures';
import { clock } from '@/lib/clock';

export interface FakeRunnerOptions {
  emit: (event: PipelineEvent) => void;
  /** Milliseconds between streamed lines. */
  pace: number;
  /** Hold the run at this step forever (for screenshots of a live run). */
  holdAt?: string;
  /** Capture posters fails the first time it runs. */
  postersFail: boolean;
}

export interface RunStore {
  runs: RunRecord[];
  logs: Map<number, LogEntry[]>;
}

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

export function createFakeRunner(store: RunStore, options: FakeRunnerOptions) {
  let active: { id: string; cancelled: boolean } | null = null;
  let postersFail = options.postersFail;

  function publish(run: RunRecord) {
    store.runs = [run, ...store.runs.filter(r => r.id !== run.id)].slice(0, 20);
    options.emit({ type: 'run', run: structuredClone(run) });
  }

  function log(
    run: RunRecord,
    step: string | null,
    text: string,
    level: LogEntry['level'] = 'info',
    stream: LogEntry['stream'] = 'stdout'
  ) {
    const entry: LogEntry = { runId: run.id, step, stream, level, text, atMs: clock.nowMs() };
    const list = store.logs.get(run.number) ?? [];
    list.push(entry);
    store.logs.set(run.number, list);
    options.emit({ type: 'log', entry });
  }

  async function execute(run: RunRecord, def: PipelineDef) {
    run.state = 'running';
    run.startedAtMs = clock.nowMs();
    publish(run);
    const planned = run.steps.filter(s => s.state !== 'skipped').length;
    log(
      run,
      null,
      `Run ${run.number} · ${planned} of ${run.steps.length} steps${run.force ? ' · forced' : ''}`,
      'info',
      'system'
    );
    let failure: string | null = null;
    for (const rec of run.steps) {
      if (rec.state === 'skipped') continue;
      if (active?.cancelled) {
        rec.state = 'cancelled';
        continue;
      }
      if (failure) {
        rec.state = 'skipped';
        rec.message = 'Not run, an earlier step failed.';
        continue;
      }
      const started = clock.nowMs();
      rec.state = 'running';
      rec.startedAtMs = started;
      publish(run);
      const forced = run.force || run.from === rec.id;
      if (!forced && (rec.id === 'build' || rec.id === 'bake')) {
        await sleep(options.pace);
        rec.state = 'cached';
        rec.durationMs = clock.nowMs() - started;
        log(run, rec.id, `${rec.name} inputs unchanged, cache hit`, 'ok', 'system');
        publish(run);
        continue;
      }
      const lines = blenderLines(rec.id).filter(
        l => rec.id !== 'posters' || postersFail || l.level !== 'error'
      );
      const hold = options.holdAt === rec.id;
      // Blender steps report how far they are, like the starter scripts'
      // `progress N/M` lines; the others stay indeterminate, as Ship does.
      const reports = rec.id !== 'ship';
      for (const [i, line] of lines.entries()) {
        if (active?.cancelled) break;
        if (hold && i === lines.length - 4) {
          // Park here until cancelled, like a long export.
          while (!active?.cancelled) await sleep(200);
          break;
        }
        await sleep(options.pace);
        log(run, rec.id, line.text, line.level ?? 'info', line.stream ?? 'stdout');
        if (reports) {
          rec.progress = (i + 1) / lines.length;
          publish(run);
        }
      }
      rec.durationMs = clock.nowMs() - started;
      if (active?.cancelled) {
        rec.state = 'cancelled';
        rec.message = 'Cancelled.';
        continue;
      }
      if (rec.id === 'posters' && postersFail) {
        postersFail = false;
        rec.state = 'failed';
        rec.exitCode = 1;
        rec.message = 'blender exited with code 1.';
        failure =
          'Capture posters failed. Camera "poster" is missing in scene_baked.blend. Add it, then run again from this step.';
        log(run, rec.id, `${rec.name} failed: blender exited with code 1.`, 'error', 'system');
      } else {
        rec.state = 'done';
        rec.exitCode = 0;
        log(
          run,
          rec.id,
          `${rec.name} done in ${(rec.durationMs / 1000).toFixed(1)}s`,
          'ok',
          'system'
        );
      }
      publish(run);
    }
    void def;
    run.finishedAtMs = clock.nowMs();
    if (active?.cancelled) {
      run.state = 'cancelled';
      run.message = 'Cancelled.';
      log(run, null, `Run ${run.number} cancelled`, 'error', 'system');
    } else if (failure) {
      run.state = 'failed';
      run.message = failure;
      log(run, null, `Run ${run.number} failed: ${failure}`, 'error', 'system');
    } else {
      run.state = 'done';
      const cached = run.steps.filter(s => s.state === 'cached').length;
      log(
        run,
        null,
        `Run ${run.number} done in ${((run.finishedAtMs - (run.startedAtMs ?? 0)) / 1000).toFixed(1)}s, ${cached} cached`,
        'ok',
        'system'
      );
    }
    active = null;
    publish(run);
  }

  return {
    get activeId() {
      return active?.id ?? null;
    },
    submit(project: string, def: PipelineDef, from: string | null, force: boolean): RunRecord {
      if (active)
        throw { code: 'invalid', message: 'A run is already going. Mock mode runs one at a time.' };
      const number = Math.max(0, ...store.runs.map(r => r.number)) + 1;
      const fromIndex = from ? def.steps.findIndex(s => s.id === from) : -1;
      const steps: StepRecord[] = def.steps.map((s, i) => ({
        id: s.id,
        name: s.name,
        state: fromIndex > i ? 'skipped' : 'waiting',
        startedAtMs: null,
        durationMs: null,
        exitCode: null,
        message: fromIndex > i ? 'Not part of this run.' : null,
        progress: null,
      }));
      const run: RunRecord = {
        id: `${project}-run-${number}`,
        number,
        project,
        from,
        force,
        state: 'queued',
        queuedAtMs: clock.nowMs(),
        startedAtMs: null,
        finishedAtMs: null,
        steps,
        message: null,
      };
      active = { id: run.id, cancelled: false };
      store.logs.set(number, []);
      publish(run);
      void execute(run, def);
      return structuredClone(run);
    },
    cancel(id: string) {
      if (!active || active.id !== id)
        throw { code: 'not-found', message: `Could not find the run ${id}.` };
      active.cancelled = true;
    },
  };
}

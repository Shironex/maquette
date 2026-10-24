import { describe, expect, it } from 'vitest';
import type { Asset, LogEntry, RunRecord } from '@/ipc/client';
import { pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import {
  MAX_LOG_LINES,
  applyEvent,
  failedStep,
  formatDuration,
  logRows,
  nodeViews,
  runStatusLine,
  runSummary,
  stageViews,
} from './pipeline';

const NOW = 1_800_000_000_000;

function view(runs = runHistory(NOW)) {
  return pipelineView('moon-island', '/w/projects/moon-island', runs, []);
}

function newRun(state: RunRecord['state']): RunRecord {
  const [latest] = runHistory(NOW);
  return {
    ...latest!,
    id: 'moon-island-run-25',
    number: 25,
    state,
    message: null,
    steps: latest!.steps.map(s => ({ ...s, state: 'waiting', durationMs: null, message: null })),
  };
}

function entry(runId: string, text: string): LogEntry {
  return { runId, step: 'export', stream: 'stdout', level: 'info', text, atMs: NOW };
}

describe('applyEvent', () => {
  it('queues, starts and finishes a run', () => {
    let v = view();
    v = applyEvent(v, { type: 'run', run: newRun('queued') });
    expect(v.queued.map(r => r.number)).toEqual([25]);
    v = applyEvent(v, { type: 'run', run: newRun('running') });
    expect(v.queued).toEqual([]);
    expect(v.active).toBe('moon-island-run-25');
    expect(v.runs[0]!.number).toBe(25);
    expect(v.log).toEqual([]);
    v = applyEvent(v, { type: 'log', entry: entry('moon-island-run-25', 'Blender 5.2') });
    expect(v.log).toHaveLength(1);
    v = applyEvent(v, { type: 'run', run: newRun('done') });
    expect(v.active).toBeNull();
    expect(v.runs).toHaveLength(3);
    expect(v.log).toHaveLength(1);
  });

  it('ignores other projects and logs of older runs', () => {
    const v = view();
    expect(applyEvent(v, { type: 'run', run: { ...newRun('running'), project: 'other' } })).toBe(v);
    expect(applyEvent(v, { type: 'log', entry: entry('moon-island-run-23', 'old') })).toBe(v);
  });

  it('caps the live log', () => {
    let v = applyEvent(view(), { type: 'run', run: newRun('running') });
    v = {
      ...v,
      log: Array.from({ length: MAX_LOG_LINES }, (_, i) => entry('moon-island-run-25', String(i))),
    };
    v = applyEvent(v, { type: 'log', entry: entry('moon-island-run-25', 'last') });
    expect(v.log).toHaveLength(MAX_LOG_LINES);
    expect(v.log.at(-1)!.text).toBe('last');
    expect(v.log[0]!.text).toBe('1');
  });
});

describe('nodeViews', () => {
  it('shows the failed run with cached, done and skipped steps', () => {
    const nodes = nodeViews(view());
    expect(nodes.build).toMatchObject({ state: 'cached' });
    expect(nodes.export).toMatchObject({ state: 'done', duration: '17.9s' });
    expect(nodes.posters).toMatchObject({ state: 'failed', detail: 'blender exited with code 1.' });
    // Run 24 skipped Ship; run 23 did it.
    expect(nodes.ship).toMatchObject({ state: 'cached', label: 'Skipped', duration: '0.9s' });
  });

  it('falls back to "failed in 24" for a step skipped after a failure', () => {
    const run = newRun('running');
    run.steps = run.steps.map(s => (s.id === 'posters' ? { ...s, state: 'skipped' } : s));
    run.steps[2] = { ...run.steps[2]!, state: 'running' };
    const nodes = nodeViews(view([run, ...runHistory(NOW)]));
    expect(nodes.export).toEqual({ state: 'running', label: 'Running' });
    expect(nodes.posters).toEqual({ state: 'waiting', label: 'failed in 24' });
    expect(nodes.compress).toEqual({ state: 'waiting', duration: 'last 6.4s' });
  });

  it('waits for every step before the first run', () => {
    const nodes = nodeViews(view([]));
    expect(Object.values(nodes).every(n => n.state === 'waiting')).toBe(true);
  });
});

describe('summaries', () => {
  it('formats durations', () => {
    expect(formatDuration(null)).toBeUndefined();
    expect(formatDuration(400)).toBe('0.4s');
    expect(formatDuration(72_000)).toBe('1m 12s');
  });

  it('describes runs', () => {
    const [r24, r23] = runHistory(NOW);
    expect(runStatusLine(undefined)).toBe('No runs yet');
    expect(runStatusLine(r24)).toBe('Run 24 failed at Capture posters');
    expect(runStatusLine(r23)).toBe('Run 23 done');
    const live = newRun('running');
    live.steps[2] = { ...live.steps[2]!, state: 'running' };
    expect(runStatusLine(live)).toBe('Run 25 · step 3 of 6');
    expect(runSummary(r24!)).toBe('2 done · 2 cached · 1 failed');
    expect(failedStep(r24)).toBe('posters');
    expect(failedStep(r23)).toBeNull();
  });

  it('filters log rows by step and keeps runner lines', () => {
    const log: LogEntry[] = [
      { ...entry('r', 'a'), step: null },
      entry('r', 'b'),
      { ...entry('r', 'c'), step: 'posters' },
    ];
    expect(logRows(log, 'all')).toHaveLength(3);
    expect(logRows(log, 'export').map(r => r.text)).toEqual(['a', 'b']);
  });
});

describe('stageViews', () => {
  const asset = (stage: Asset['stage']) => ({ stage }) as Asset;

  it('marks stages up to the furthest asset', () => {
    const states = stageViews([asset('refs'), asset('blender')], undefined).map(s => s.state);
    expect(states).toEqual(['done', 'done', 'done', 'current', 'todo', 'todo']);
  });

  it('shows a live or failed run on the Web stage', () => {
    const running = stageViews([asset('blender')], newRun('running'));
    expect(running.find(s => s.id === 'web')!.state).toBe('running');
    const failed = stageViews([asset('web')], runHistory(NOW)[0]);
    expect(failed.find(s => s.id === 'web')!.state).toBe('failed');
    const early = stageViews([asset('concept')], runHistory(NOW)[0]);
    expect(early.find(s => s.id === 'web')!.state).toBe('todo');
  });
});

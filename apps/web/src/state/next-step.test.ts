import { describe, expect, it } from 'vitest';
import type { Asset, ToolReport } from '@/ipc/client';
import { blenderTrouble } from './toolchain';
import { nextStep, type NextFacts } from './next-step';

const asset = (id: string, stage: Asset['stage'], versions = 0) =>
  ({ id, name: id, stage, versions: Array(versions).fill({}) }) as unknown as Asset;

const facts = (over: Partial<NextFacts> = {}): NextFacts => ({
  blender: null,
  inbox: 0,
  running: false,
  tipsShowing: false,
  target: 'website',
  hasRepo: true,
  assets: [],
  sending: new Set(),
  lastRun: undefined,
  ...over,
});

describe('the next step', () => {
  it('puts a missing Blender before everything, even a run', () => {
    expect(nextStep(facts({ blender: 'missing', inbox: 3, running: true }))?.id).toBe('blender');
  });

  it('says a copy that really fails did not run, on the same way to Toolchain', () => {
    const step = nextStep(facts({ blender: 'broken', inbox: 3, running: true }));
    expect(step?.id).toBe('blender-broken');
    expect(step?.action).toBe('toolchain');
    expect(step?.sentence).toMatch(/did not run/);
    expect(step?.sentence).not.toMatch(/missing/);
  });

  it('says a gone saved path only when nothing else waits, since builds still work', () => {
    const stale = { blender: 'stale-path', assets: [asset('a', 'blender', 1)] } as const;
    const step = nextStep(facts({ ...stale, lastRun: { state: 'done' } }));
    expect(step?.id).toBe('blender-stale');
    expect(step?.action).toBe('toolchain');
    expect(step?.sentence).toMatch(/saved Blender path is gone/);
    expect(step?.sentence).not.toMatch(/did not run|missing|install/i);
    expect(nextStep(facts({ ...stale, inbox: 2 }))?.id).toBe('inbox');
    expect(nextStep(facts(stale))?.id).toBe('run');
    expect(nextStep(facts({ ...stale, running: true }))).toBeNull();
  });

  it('reads the report: no copy is missing, a failing copy broken, a fallback a stale path', () => {
    const report = (status: ToolReport['status'], warning: string | null = null) =>
      ({ id: 'blender', status, warning }) as ToolReport;
    expect(blenderTrouble(undefined)).toBeNull();
    expect(blenderTrouble(report('found'))).toBeNull();
    expect(blenderTrouble(report('missing'))).toBe('missing');
    expect(blenderTrouble(report('missing', 'stale'))).toBe('missing');
    expect(blenderTrouble(report('broken'))).toBe('broken');
    expect(blenderTrouble(report('broken', 'stale'))).toBe('broken');
    expect(blenderTrouble(report('found', 'stale'))).toBe('stale-path');
  });

  it('stays quiet during a run', () => {
    expect(nextStep(facts({ running: true, inbox: 2 }))).toBeNull();
  });

  it('counts the inbox before the project', () => {
    expect(nextStep(facts({ inbox: 1, assets: [asset('a', 'refs')] }))?.sentence).toMatch(
      /^1 model waits/
    );
    expect(nextStep(facts({ inbox: 2 }))?.sentence).toMatch(/^2 models wait/);
  });

  it('leaves an empty Board to the tips, then starts it per target', () => {
    expect(nextStep(facts({ tipsShowing: true }))).toBeNull();
    expect(nextStep(facts())?.action).toBe('lab');
    expect(nextStep(facts({ target: 'game' }))?.action).toBe('import');
  });

  it('sends the refs with no model and no send on its way', () => {
    const step = nextStep(
      facts({
        assets: [asset('a', 'refs'), asset('b', 'refs'), asset('c', 'refs', 1)],
        sending: new Set(['b']),
      })
    );
    expect(step?.action).toBe('send');
    expect(step?.assets).toEqual(['a']);
    expect(step?.sentence).toMatch(/^a has refs/);
  });

  it('says nothing about refs a send already covers', () => {
    expect(nextStep(facts({ assets: [asset('a', 'refs')], sending: new Set(['a']) }))).toBeNull();
  });

  it('asks for picks while only concepts exist', () => {
    expect(nextStep(facts({ assets: [asset('a', 'concept')] }))?.id).toBe('concept');
  });

  it('walks a project with models from run to repo to done', () => {
    const assets = [asset('a', 'blender', 1)];
    expect(nextStep(facts({ assets }))?.id).toBe('run');
    const done = { state: 'done' } as const;
    expect(nextStep(facts({ assets, lastRun: { state: 'failed' } }))?.id).toBe('run-failed');
    expect(nextStep(facts({ assets, lastRun: done, hasRepo: false }))?.id).toBe('repo');
    expect(nextStep(facts({ assets, lastRun: done }))).toBeNull();
  });

  it('drops the repo step as soon as a repo is set, before another run', () => {
    const assets = [asset('a', 'blender', 1)];
    const lastRun = { state: 'done' } as const;
    expect(nextStep(facts({ assets, lastRun, hasRepo: false }))?.id).toBe('repo');
    expect(nextStep(facts({ assets, lastRun, hasRepo: true }))).toBeNull();
  });

  it('offers a cancelled run again and never calls it done', () => {
    const lastRun = { state: 'cancelled' } as const;
    for (const hasRepo of [true, false]) {
      const step = nextStep(facts({ assets: [asset('a', 'blender', 1)], lastRun, hasRepo }));
      expect(step?.id).toBe('run-cancelled');
      expect(step?.action).toBe('pipeline');
      expect(step?.sentence).toMatch(/cancelled/);
      expect(step?.sentence).not.toMatch(/done/);
    }
  });

  it('names the game repo for a game', () => {
    const step = nextStep(
      facts({
        target: 'game',
        hasRepo: false,
        assets: [asset('a', 'ship', 1)],
        lastRun: { state: 'done' },
      })
    );
    expect(step?.sentence).toContain('game repo');
  });
});

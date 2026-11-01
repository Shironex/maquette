import { describe, expect, it } from 'vitest';
import type { Asset } from '@/ipc/client';
import { nextStep, type NextFacts } from './next-step';

const asset = (id: string, stage: Asset['stage'], versions = 0) =>
  ({ id, name: id, stage, versions: Array(versions).fill({}) }) as unknown as Asset;

const facts = (over: Partial<NextFacts> = {}): NextFacts => ({
  blenderMissing: false,
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
    expect(nextStep(facts({ blenderMissing: true, inbox: 3, running: true }))?.id).toBe('blender');
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
    const done = { state: 'done', notShipped: false } as const;
    expect(nextStep(facts({ assets, lastRun: { ...done, state: 'failed' } }))?.id).toBe(
      'run-failed'
    );
    expect(nextStep(facts({ assets, lastRun: done, hasRepo: false }))?.id).toBe('repo');
    expect(nextStep(facts({ assets, lastRun: { ...done, notShipped: true } }))?.id).toBe('repo');
    expect(nextStep(facts({ assets, lastRun: done }))).toBeNull();
  });

  it('names the game repo for a game', () => {
    const step = nextStep(
      facts({
        target: 'game',
        hasRepo: false,
        assets: [asset('a', 'ship', 1)],
        lastRun: { state: 'done', notShipped: true },
      })
    );
    expect(step?.sentence).toContain('game repo');
  });
});

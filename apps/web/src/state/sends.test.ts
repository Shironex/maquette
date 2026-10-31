import { describe, expect, it } from 'vitest';
import type { JobState, Provider, ProviderJob } from '@/ipc/client';
import { sendCounts, sendsNeedYou, sendsOpensOn, sendsShowTabs, sendsSummary } from './sends';

const job = (provider: Provider, state: JobState, id = `${provider}-${state}`) =>
  ({ id, provider, state, assetName: id, batch: null }) as unknown as ProviderJob;

describe('sendCounts', () => {
  it('counts a By hand send waiting for its model once', () => {
    const counts = sendCounts([job('by-hand', 'waiting', 'a'), job('by-hand', 'waiting', 'b')]);
    expect(counts).toMatchObject({ inFlight: 2, hand: 2, api: 0, toMake: 2 });
  });

  it('totals both kinds and leaves finished sends out', () => {
    const counts = sendCounts([
      job('by-hand', 'waiting'),
      job('tripo', 'running'),
      job('meshy', 'approval'),
      job('tripo', 'failed'),
      job('by-hand', 'inbox'),
      job('meshy', 'cancelled'),
    ]);
    expect(counts).toEqual({ inFlight: 3, api: 2, hand: 1, toMake: 1, approval: 1, failed: 1 });
  });
});

describe('sendsOpensOn', () => {
  it('opens By hand only when By hand is all that runs', () => {
    expect(sendsOpensOn(sendCounts([job('by-hand', 'waiting')]))).toBe('hand');
    expect(sendsOpensOn(sendCounts([job('tripo', 'running')]))).toBe('queue');
    expect(sendsOpensOn(sendCounts([job('by-hand', 'waiting'), job('tripo', 'queued')]))).toBe(
      'queue'
    );
  });
});

describe('sendsNeedYou', () => {
  it('is true for a model to make or a send to approve, not for a running one', () => {
    expect(sendsNeedYou(sendCounts([job('by-hand', 'waiting')]))).toBe(true);
    expect(sendsNeedYou(sendCounts([job('tripo', 'approval')]))).toBe(true);
    expect(sendsNeedYou(sendCounts([job('tripo', 'running')]))).toBe(false);
  });
});

describe('sendsSummary', () => {
  it('says what waits on what', () => {
    expect(sendsSummary(sendCounts([job('tripo', 'running')]))).toBe('1 send in flight');
    expect(
      sendsSummary(
        sendCounts([job('by-hand', 'waiting'), job('meshy', 'approval'), job('tripo', 'failed')])
      )
    ).toBe('2 sends in flight: 1 to make by hand, 1 waiting for approval, 1 failed');
  });
});

describe('sendsShowTabs', () => {
  const hand = job('by-hand', 'waiting');
  const paid = job('tripo', 'running');
  it('shows them with both kinds under way', () => {
    expect(sendsShowTabs([hand, paid], 'queue')).toBe(true);
    expect(sendsShowTabs([hand, paid], 'hand')).toBe(true);
  });
  it('hides them when the open list is the only kind under way', () => {
    expect(sendsShowTabs([paid], 'queue')).toBe(false);
    expect(sendsShowTabs([hand], 'hand')).toBe(false);
  });
  it('keeps them when the other list is the only kind under way', () => {
    expect(sendsShowTabs([paid], 'hand')).toBe(true);
    expect(sendsShowTabs([hand], 'queue')).toBe(true);
  });
  it('with nothing under way, shows By hand only when it lists something', () => {
    expect(sendsShowTabs([job('tripo', 'failed')], 'queue')).toBe(false);
    expect(sendsShowTabs([job('by-hand', 'inbox')], 'queue')).toBe(true);
    expect(sendsShowTabs([], 'queue')).toBe(false);
  });
});

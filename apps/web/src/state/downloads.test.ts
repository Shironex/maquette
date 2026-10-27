import { describe, expect, it } from 'vitest';
import type { Arrival, PendingHandoff, WatchStatus } from '@/ipc/client';
import { arrivalNotice, askingArrivals, sendOptions, watchLine } from './downloads';

const arrival = (over: Partial<Arrival> = {}): Arrival => ({
  id: 'a1',
  path: '/Users/maker/Downloads/model.glb',
  name: 'model.glb',
  inner: null,
  bytes: 1000,
  atMs: 0,
  state: 'unmatched',
  job: null,
  project: null,
  assetName: null,
  reason: '2 sends are waiting',
  inboxFile: null,
  ...over,
});

const pending = (job: string, project: string, assetName: string): PendingHandoff => ({
  job,
  project,
  asset: job.split('-')[0]!,
  assetName,
  service: 'tripo',
  expectedName: `maq_${job}`,
  handoff: null,
  credits: 30,
  createdAtMs: 0,
});

const status = (over: Partial<WatchStatus> = {}): WatchStatus => ({
  enabled: true,
  folder: '/Users/maker/Downloads',
  custom: false,
  watching: true,
  error: null,
  pending: [],
  arrivals: [],
  ...over,
});

describe('Downloads watcher words', () => {
  it('says quietly when a model is in, and asks when it cannot tell', () => {
    const imported = arrivalNotice(
      arrival({
        state: 'imported',
        assetName: 'Guardian statue',
        reason: 'Named for Guardian statue',
      })
    );
    expect(imported).toMatchObject({ status: 'done', title: 'Guardian statue is in the Inbox' });
    expect(imported?.ask).toBe(false);
    const unmatched = arrivalNotice(arrival());
    expect(unmatched?.ask).toBe(true);
    expect(unmatched?.detail).toContain('which asset is this?');
    expect(arrivalNotice(arrival({ state: 'needs-glb', reason: 'Export a GLB' }))?.ask).toBe(false);
    expect(arrivalNotice(arrival({ state: 'failed', reason: 'Broken' }))?.status).toBe('failed');
    expect(arrivalNotice(arrival({ state: 'dismissed' }))).toBeNull();
  });

  it('keeps only the downloads still waiting for an answer', () => {
    const list = [
      arrival({ id: 'a' }),
      arrival({ id: 'b', state: 'imported' }),
      arrival({ id: 'c', state: 'failed' }),
      arrival({ id: 'd', state: 'dismissed' }),
    ];
    expect(askingArrivals(status({ arrivals: list })).map(a => a.id)).toEqual(['a', 'c']);
    expect(askingArrivals(null)).toEqual([]);
  });

  it('names sends by asset, with the project when there are several', () => {
    const name = (id: string) => (id === 'moon' ? 'Frostveil' : 'Rooftop');
    const one = [pending('lantern-1', 'moon', 'Lantern'), pending('cup-2', 'moon', 'Cup')];
    expect(sendOptions(one, name).map(o => o.label)).toEqual(['Lantern', 'Cup']);
    const two = [...one, pending('chair-1', 'roof', 'Chair')];
    expect(sendOptions(two, name).map(o => o.label)).toEqual([
      'Lantern (Frostveil)',
      'Cup (Frostveil)',
      'Chair (Rooftop)',
    ]);
  });

  it('describes the watch in one line', () => {
    expect(watchLine(null).tone).toBe('mute');
    expect(watchLine(status({ enabled: false })).text).toMatch(/^Off\./);
    expect(watchLine(status({ error: 'Gone', watching: false }))).toEqual({
      text: 'Gone',
      tone: 'error',
    });
    expect(watchLine(status()).text).toBe(
      'Watching. Nothing waits for a model, so nothing is read.'
    );
    expect(watchLine(status({ pending: [pending('lantern-1', 'moon', 'Lantern')] })).text).toBe(
      'Watching for 1 By hand send.'
    );
  });
});

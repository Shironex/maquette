import { describe, expect, it } from 'vitest';
import type { Arrival, Asset, PendingHandoff, WatchStatus } from '@/ipc/client';
import { stageLabels } from './game';
import {
  arrivalNotice,
  askingArrivals,
  assetChoices,
  lookingArrivals,
  waitingArrivals,
  watchLine,
} from './downloads';

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
  asset: null,
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
  image: `refs/${job.split('-')[0]!}.png`,
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

  it('lists the assets to attach to, the waiting sends first, by search', () => {
    const asset = (id: string, name: string, versions = 0): Asset => ({
      id,
      name,
      kind: 'prop',
      stage: versions ? 'blender' : 'refs',
      status: 'queued',
      versions: Array.from({ length: versions }, (_, i) => ({
        version: i + 1,
        file: `models/${id}_v${i + 1}.glb`,
        createdAt: '',
        stats: null,
      })),
      thumbnail: `thumbs/${id}.png`,
      createdAt: '',
      updatedAt: '',
    });
    const assets = [asset('orb', 'Orb shrine', 2), asset('lantern', 'Lantern post')];
    const sends = [pending('lantern-1', 'moon', 'Lantern post'), pending('cup-1', 'roof', 'Cup')];
    const all = assetChoices(assets, sends, 'moon', stageLabels('website'));
    expect(all.map(c => [c.name, c.state, c.picture])).toEqual([
      ['Lantern post', 'Waiting for its Tripo model', 'refs/lantern.png'],
      ['Orb shrine', 'Blender, v2', 'thumbs/orb.png'],
    ]);
    expect(assetChoices(assets, sends, 'moon', stageLabels('website'), 'orb door')).toHaveLength(0);
    expect(
      assetChoices(assets, sends, 'moon', stageLabels('website'), 'orb shr').map(c => c.name)
    ).toEqual(['Orb shrine']);
    expect(assetChoices([asset('cup', 'Cup')], [], 'moon', stageLabels('website'))[0]?.state).toBe(
      'Refs, no model yet'
    );
  });

  it('asks for a look at a close name before anything goes in', () => {
    const look = arrival({
      state: 'review',
      assetName: 'Lantern post',
      reason: 'Name close to Lantern post',
    });
    expect(arrivalNotice(look)).toMatchObject({
      title: 'Is this Lantern post?',
      ask: true,
      action: 'Look',
    });
    expect(
      arrivalNotice(
        arrival({
          state: 'attached',
          assetName: 'Lantern post',
          reason: 'You attached it to Lantern post',
        })
      )
    ).toMatchObject({
      status: 'done',
      title: 'model.glb is in the Inbox for Lantern post',
      ask: false,
    });
    const list = status({ arrivals: [arrival({ id: 'u' }), look] });
    expect(lookingArrivals(list)).toEqual([look]);
    expect(waitingArrivals(list).map(a => a.id)).toEqual(['a1', 'u']);
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

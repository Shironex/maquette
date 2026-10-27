import { describe, expect, it } from 'vitest';
import type { Asset, ProviderJob, RemoveOutcome } from '@/ipc/client';
import { assetsLabel, removePlan, removedNotice, restoredNotice, stopLabel } from './trash';

const asset = (id: string, name: string) => ({ id, name }) as Asset;
const job = (id: string, assetId: string, state: ProviderJob['state'], provider = 'tripo') =>
  ({ id, asset: assetId, state, provider }) as ProviderJob;
const outcome = (over: Partial<RemoveOutcome>): RemoveOutcome => ({
  asset: 'clay-bowl',
  name: 'Clay bowl',
  state: 'removed',
  reason: null,
  trash: 'clay-bowl-1',
  trashPath: 'trash/moon/clay-bowl-1',
  files: 3,
  stopped: [],
  warnings: [],
  ...over,
});

describe('removePlan', () => {
  it('stops the sends waiting for a model and waits for the ones being made', () => {
    const plan = removePlan(
      [asset('clay-bowl', 'Clay bowl'), asset('lamp', 'Lamp')],
      [
        job('clay-bowl-1', 'clay-bowl', 'waiting', 'by-hand'),
        job('clay-bowl-2', 'clay-bowl', 'approval'),
        job('lamp-1', 'lamp', 'running', 'meshy'),
        job('lamp-0', 'lamp', 'accepted'),
        job('other-1', 'other', 'waiting'),
      ]
    );
    expect(plan.stops.map(j => j.id)).toEqual(['clay-bowl-1', 'clay-bowl-2']);
    expect(plan.running.map(j => j.id)).toEqual(['lamp-1']);
    expect(stopLabel(plan.stops[0]!)).toBe('waiting for your model, made by hand');
    expect(stopLabel(plan.stops[1]!)).toBe('waiting for approval to send to Tripo');
    expect(stopLabel(job('x', 'x', 'queued', 'meshy'))).toBe('queued for Meshy, nothing sent yet');
  });

  it('names one asset, or counts them', () => {
    expect(assetsLabel([{ name: 'Clay bowl' }])).toBe('Clay bowl');
    expect(assetsLabel(Array.from({ length: 10 }, () => ({ name: 'x' })))).toBe('10 assets');
  });
});

describe('removedNotice', () => {
  it('says where the files went', () => {
    expect(removedNotice([outcome({})])).toEqual({
      status: 'done',
      title: 'Removed Clay bowl',
      detail: 'Its files are in the workspace trash; Undo or Recently removed puts it back.',
    });
  });

  it('counts a partial remove and gives the reasons', () => {
    const notice = removedNotice([
      outcome({}),
      outcome({ asset: 'lamp', name: 'Lamp', state: 'refused', reason: 'Tripo is busy.' }),
      outcome({ asset: 'gone', name: null, state: 'not-found' }),
    ]);
    expect(notice.status).toBe('info');
    expect(notice.title).toBe('Removed 1 of 3 assets');
    expect(notice.detail).toContain('Tripo is busy. 1 was already gone.');
  });

  it('fails when nothing went, and says when nothing was there', () => {
    expect(
      removedNotice([outcome({ state: 'refused', reason: 'Wait for the build.' })])
    ).toMatchObject({ status: 'failed', title: 'Could not remove Clay bowl' });
    expect(removedNotice([outcome({ state: 'not-found' })])).toEqual({
      status: 'info',
      title: 'Nothing to remove',
      detail: '1 was already gone.',
    });
  });
});

describe('restoredNotice', () => {
  it('says what came back and why the rest did not', () => {
    expect(restoredNotice(['Clay bowl'], []).title).toBe('Restored Clay bowl');
    expect(restoredNotice(['A', 'B'], []).title).toBe('Restored 2 assets');
    expect(restoredNotice([], ['Lamp: taken.'])).toMatchObject({
      status: 'failed',
      detail: 'Lamp: taken.',
    });
    expect(restoredNotice(['A'], ['B: taken.']).title).toBe('Restored 1 of 2 assets');
  });
});

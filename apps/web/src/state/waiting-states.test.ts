import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type Asset, type ConceptJob, type RenderRecord } from '@/ipc/client';
import { readLastRun } from '@/lib/waiting';
import { jobMeta, lastJobMs } from './lab';
import { openInBlender } from './open-in-blender';
import { finishedRender, renderBatchWait, renderWait } from './renders';
import { wantsPreview } from './thumbs';
import { useAppStore } from './use-app-store';
import { useGameStore } from './use-game-store';
import { useRenderStore } from './use-render-store';

const job = (state: ConceptJob['state'], started: number | null, finished: number | null) =>
  ({ state, startedAtMs: started, finishedAtMs: finished }) as ConceptJob;

const record = (over: Partial<RenderRecord>): RenderRecord =>
  ({
    id: 'r',
    state: 'done',
    createdAt: '2026-11-01T10:00:00.000Z',
    finishedAt: '2026-11-01T10:00:14.000Z',
    settings: { size: 1024 },
    sheet: null,
    items: [{ files: [{ style: 'clay', state: 'done' }] }],
    ...over,
  }) as unknown as RenderRecord;

describe('a running image reads like the finished ones', () => {
  it('says only a real past job, and nothing without one', () => {
    expect(lastJobMs([job('running', 1, null), job('failed', 1, 9)])).toBeNull();
    const last = lastJobMs([job('done', 0, 58_000), job('done', 60_000, 124_000)]);
    expect(last).toBe(64_000);
    expect(jobMeta(job('running', 1000, null), 43_000, last)).toBe(
      'Generating · 42s · last 1m 04s'
    );
    expect(jobMeta(job('running', 1000, null), 43_000)).toBe('Generating · 42s');
  });
});

describe('a render says what a past one took', () => {
  beforeEach(() => window.localStorage.clear());

  it('remembers a finished single render under its style and size', () => {
    useRenderStore.getState().onChanged(record({}));
    const settings = { size: 1024 } as RenderRecord['settings'];
    expect(readLastRun(renderWait('clay', settings))).toBe(14_000);
    expect(readLastRun(renderBatchWait(1))).toBe(14_000);
  });

  it('remembers nothing of a render still going, stopped or with a failure', () => {
    expect(finishedRender(record({ state: 'running', finishedAt: null }))).toBeNull();
    expect(finishedRender(record({ state: 'stopped' }))).toBeNull();
    const failed = { files: [{ style: 'clay', state: 'failed' }] };
    expect(finishedRender(record({ items: [failed] as RenderRecord['items'] }))).toBeNull();
  });
});

describe('Open in Blender', () => {
  it('answers the click first and says when it could not', async () => {
    let fail: (reason: unknown) => void = () => {};
    const opened = openInBlender(
      'Lantern',
      () =>
        new Promise((_, reject) => {
          fail = reject;
        })
    );
    expect(useAppStore.getState().notice?.title).toBe('Opening Lantern in Blender');
    fail({ code: 'x', message: 'Blender is gone' });
    await opened;
    expect(useAppStore.getState().notice?.title).toBe('Could not open Blender');
  });
});

describe('Render previews on the cards', () => {
  const asset = {
    id: 'a',
    thumbnail: null,
    versions: [{ version: 1, file: 'models/a_v1.glb' }],
  } as unknown as Asset;

  afterEach(() => {
    vi.restoreAllMocks();
    useGameStore.setState({ waiting: 0, waitingFor: null, failedFiles: [], failedPreviews: 0 });
  });

  it('wants a preview only for a renderable model with no picture of its own', () => {
    expect(wantsPreview(asset)).toBe(true);
    expect(wantsPreview({ ...asset, thumbnail: 'thumbs/a.png' })).toBe(false);
    expect(wantsPreview({ ...asset, thumbnail: 'refs/a.png' })).toBe(true);
    expect(wantsPreview({ ...asset, versions: [] })).toBe(false);
  });

  it('keeps the queued count from the ask until each render reports', async () => {
    const project = { id: 'p', assets: [asset] } as never;
    vi.spyOn(commands, 'thumbnailRenderMissing').mockResolvedValue({
      asked: 1,
      rendering: 1,
      failed: 0,
      reason: null,
      project,
    });
    await useGameStore.getState().renderPreviews('p');
    expect(useGameStore.getState().waiting).toBe(1);
    expect(useGameStore.getState().waitingFor).toBe('p');
  });
});

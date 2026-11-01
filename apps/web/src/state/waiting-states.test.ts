import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  commands,
  type Asset,
  type ConceptJob,
  type RenderRecord,
  type RigRecord,
} from '@/ipc/client';
import { readLastRun } from '@/lib/waiting';
import { jobMeta, lastJobMs } from './lab';
import { openInBlender } from './open-in-blender';
import { runStartLine, runWait } from './rig';
import { finishedRender, renderBatchWait, renderWait } from './renders';
import { wantsPreview } from './thumbs';
import { useAppStore } from './use-app-store';
import { useGameStore } from './use-game-store';
import { useInboxStore } from './use-inbox-store';
import { useRenderStore } from './use-render-store';
import { RIG_OPEN_WAIT, runKindOf, useRigStore } from './use-rig-store';

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
    items: [{ files: [{ style: 'clay', state: 'done', seconds: 14 }] }],
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
    expect(readLastRun(renderBatchWait(record({})))).toBe(14_000);
  });

  it('counts the work, not the wait behind another render or a stop', () => {
    const queued = record({ createdAt: '2026-11-01T09:00:00.000Z' });
    expect(finishedRender(queued)?.ms).toBe(14_000);
  });

  it('remembers nothing it did not measure', () => {
    const unmeasured = { files: [{ style: 'clay', state: 'done', seconds: 0 }] };
    expect(finishedRender(record({ items: [unmeasured] as RenderRecord['items'] }))).toBeNull();
  });

  it('keeps different work under different names', () => {
    const s = { size: 1024, look: 'clay', length: 3, width: 480 } as RenderRecord['settings'];
    expect(renderWait('turntable', s)).not.toBe(renderWait('turntable', { ...s, look: 'toon' }));
    expect(renderWait('turntable', s)).not.toBe(renderWait('turntable', { ...s, width: 960 }));
    expect(renderWait('contact-sheet', s)).not.toBe(
      renderWait('contact-sheet', { ...s, look: 'toon' })
    );
    const clay = record({ settings: s });
    const toon = record({
      settings: s,
      items: [{ files: [{ style: 'toon', state: 'done', seconds: 1 }] }] as RenderRecord['items'],
    });
    expect(renderBatchWait(clay)).not.toBe(renderBatchWait(toon));
    expect(renderBatchWait(clay)).not.toBe(
      renderBatchWait(record({ settings: { ...s, size: 2048 } }))
    );
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
    useGameStore.setState({ queued: [], waitingFor: null, failedPreviews: 0 });
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
      renderingFiles: ['/ws/projects/p/models/a_v1.glb'],
      failed: 0,
      reason: null,
      project,
    });
    await useGameStore.getState().renderPreviews('p');
    expect(useGameStore.getState().queued).toHaveLength(1);
    expect(useGameStore.getState().waitingFor).toBe('p');
  });
});

describe('the inbox read', () => {
  beforeEach(() => window.localStorage.clear());
  afterEach(() => vi.restoreAllMocks());

  it('says it failed, and clears that when it reads again', async () => {
    useInboxStore.setState({ snapshot: null, loadFailed: false });
    const list = vi
      .spyOn(commands, 'inboxList')
      .mockRejectedValueOnce({ code: 'x', message: 'no' });
    await useInboxStore.getState().load();
    expect(useInboxStore.getState().loadFailed).toBe(true);
    list.mockResolvedValueOnce({ items: [] } as never);
    await useInboxStore.getState().load();
    expect(useInboxStore.getState().loadFailed).toBe(false);
  });
});

describe('the rig doctor opening', () => {
  const rec = (over: Partial<RigRecord> = {}) =>
    ({ id: 'c', name: 'Fox', state: 'ready', route: 'humanoid', ...over }) as RigRecord;
  const target = { inbox: 'x' } as never;
  const held = <T>() => {
    let resolve: (v: T) => void = () => {};
    let reject: (e: unknown) => void = () => {};
    const promise = new Promise<T>((res, rej) => {
      resolve = res;
      reject = rej;
    });
    return { promise, resolve, reject };
  };

  beforeEach(() => {
    window.localStorage.clear();
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
    useAppStore.setState({ view: 'board', notice: null });
    useRigStore.setState({ record: null, opening: null, open: [], from: 'inbox' });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('leaves the person where they went when the open fails, and still tells them', async () => {
    const open = held<RigRecord>();
    vi.spyOn(commands, 'rigOpen').mockReturnValue(open.promise);
    const going = useRigStore.getState().show(target, 'Fox');
    expect(useAppStore.getState().view).toBe('inbox');
    useAppStore.setState({ view: 'library' });
    open.reject({ code: 'x', message: 'no' });
    await going;
    expect(useAppStore.getState().view).toBe('library');
    expect(useAppStore.getState().notice?.title).toBe('Could not open the rig doctor');
  });

  it('tells of a failed open after Back too, and goes back only for who still waits', async () => {
    const left = held<RigRecord>();
    vi.spyOn(commands, 'rigOpen').mockReturnValue(left.promise);
    const going = useRigStore.getState().show(target, 'Fox');
    useRigStore.getState().leave();
    useAppStore.setState({ view: 'library' });
    left.reject({ code: 'x', message: 'no' });
    await going;
    expect(useAppStore.getState().view).toBe('library');
    expect(useAppStore.getState().notice?.title).toBe('Could not open the rig doctor');

    const waiting = held<RigRecord>();
    vi.mocked(commands.rigOpen).mockReturnValue(waiting.promise);
    useAppStore.setState({ view: 'board' });
    const again = useRigStore.getState().show(target, 'Fox');
    waiting.reject({ code: 'x', message: 'no' });
    await again;
    expect(useAppStore.getState().view).toBe('board');
  });

  it('remembers an open only when someone waited for it', async () => {
    const left = held<RigRecord>();
    vi.spyOn(commands, 'rigOpen').mockReturnValue(left.promise);
    const going = useRigStore.getState().show(target, 'Fox');
    useRigStore.getState().leave();
    vi.setSystemTime(1_030_000);
    left.resolve(rec());
    await going;
    expect(readLastRun(RIG_OPEN_WAIT)).toBeNull();

    const waited = held<RigRecord>();
    vi.mocked(commands.rigOpen).mockReturnValue(waited.promise);
    const again = useRigStore.getState().show(target, 'Fox');
    vi.setSystemTime(1_042_000);
    waited.resolve(rec());
    await again;
    expect(readLastRun(RIG_OPEN_WAIT)).toBe(12_000);
  });

  it('drops a case answer that comes after the person left', async () => {
    const get = held<RigRecord>();
    vi.spyOn(commands, 'rigGet').mockReturnValue(get.promise);
    const going = useRigStore.getState().showCase('c');
    useRigStore.getState().leave();
    get.resolve(rec());
    await going;
    expect(useRigStore.getState().record).toBeNull();
  });
});

describe('a rig doctor run', () => {
  const rec = (over: Partial<RigRecord> = {}) =>
    ({ id: 'run', name: 'Fox', state: 'fixed', route: 'creature', ...over }) as RigRecord;

  beforeEach(() => {
    window.localStorage.clear();
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
    useRigStore.setState({ record: rec(), open: [] });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it.each(['fix', 'round2', 'walk-proof'] as const)('times %s under its own name', async what => {
    vi.spyOn(commands, 'rigRun').mockResolvedValue(rec({ state: 'fixing' }));
    await useRigStore.getState().run(what);
    expect(runKindOf(rec({ state: 'fixing' }))).toBe(what);
    vi.setSystemTime(1_020_000);
    useRigStore.getState().onChanged(rec());
    expect(readLastRun(runWait(what, 'creature'))).toBe(20_000);
    vi.setSystemTime(1_000_000);
  });

  it('says what it starts, and only what it knows', () => {
    expect(runStartLine('walk-proof')).toBe('Starting the walk proof...');
    expect(runStartLine('round2')).toBe('Starting round 2...');
    expect(runStartLine(null)).toBe('Starting the run...');
  });

  it('remembers nothing of a run first seen part way, as after a reload', () => {
    const seen = rec({ id: 'seen', state: 'fixing', progress: { done: 9, total: 13 } as never });
    useRigStore.getState().onChanged(seen);
    expect(runKindOf(seen)).toBeNull();
    vi.setSystemTime(1_020_000);
    useRigStore.getState().onChanged(rec({ id: 'seen' }));
    for (const what of ['fix', 'round2', 'walk-proof', 'pose-test'] as const) {
      expect(readLastRun(runWait(what, 'creature'))).toBeNull();
    }
  });
});

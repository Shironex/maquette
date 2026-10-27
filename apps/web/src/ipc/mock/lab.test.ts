import { describe, expect, it } from 'vitest';
import type { ConceptEvent, LabView, SentPicks } from '../client';
import { createBackend } from './backend';

function backend(lab: string | null = null) {
  const events: ConceptEvent[] = [];
  const handle = createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    conceptMs: 5,
    lab,
    emit: e => {
      if (e.name === 'concept') events.push(e.payload);
    },
  });
  return { handle, events };
}

async function settled(handle: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>) {
  for (let i = 0; i < 200; i++) {
    const view = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    if (view.jobs.every(j => j.state !== 'queued' && j.state !== 'running')) return view;
    await new Promise(r => setTimeout(r, 5));
  }
  throw new Error('jobs did not finish');
}

describe('mock concept lab', () => {
  it('generates a few at a time and files the images under the direction', async () => {
    const { handle, events } = backend();
    await handle('workspace_save_config', {
      config: {
        ...((await handle('app_bootstrap')) as { workspace: { config: object } }).workspace.config,
        lab: { concurrency: 2, timeoutMinutes: 10 },
      },
    });
    const started = (await handle('lab_generate', {
      request: {
        project: 'frostveil',
        direction: 'Guardian statue',
        prompt: 'A lantern',
        refs: [],
        aspect: '16:9',
        count: 3,
      },
    })) as LabView;
    expect(started.jobs.filter(j => j.state === 'running')).toHaveLength(2);
    const view = await settled(handle);
    const lantern = view.directions.find(d => d.id === 'guardian-statue');
    expect(lantern?.images.map(i => i.index)).toEqual([1, 2, 3]);
    expect(events.some(e => e.type === 'direction')).toBe(true);
  });

  it('marks, then sends picks to Refs and puts the asset on the Board', async () => {
    const { handle } = backend();
    const file = 'concepts/veil-gate-kit/veil-gate-kit_02.png';
    await handle('lab_mark', {
      project: 'frostveil',
      direction: 'veil-gate-kit',
      file,
      starred: true,
      picked: true,
    });
    const sent = (await handle('lab_send_picks', {
      project: 'frostveil',
      direction: 'veil-gate-kit',
      kind: 'scene',
    })) as SentPicks;
    expect(sent.copied).toEqual(['refs/veil-gate-kit_02.png']);
    expect(sent.asset.stage).toBe('refs');
    expect(sent.project.assets?.some(a => a.id === 'veil-gate-kit')).toBe(true);
    const view = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    expect(view.refs.some(r => r.file === 'refs/veil-gate-kit_02.png')).toBe(true);
  });

  it('gives object refs of another source with the same name their own direction', async () => {
    const { handle } = backend();
    const make = (source: string) =>
      handle('lab_make_objects', {
        request: {
          project: 'frostveil',
          source,
          sourceName: 'Bench',
          objects: ['lamp'],
          notes: {},
          aspect: 'square',
          kind: 'prop',
        },
      }) as Promise<LabView>;
    await make('refs/frozen-lake-concept.png');
    await make('refs/veil-gate-kit-concept.png');
    const view = await make('refs/frozen-lake-concept.png');
    const benches = view.directions.filter(d => d.id.startsWith('bench-objects'));
    expect(benches.map(d => [d.id, d.source]).sort()).toEqual([
      ['bench-objects', 'refs/frozen-lake-concept.png'],
      ['bench-objects-2', 'refs/veil-gate-kit-concept.png'],
    ]);
    const lamps = view.jobs.filter(j => j.direction === 'bench-objects');
    expect(lamps.map(j => j.index)).toEqual([1, 2]);
  });

  it('holds two arena jobs in the running scenario and cancels them', async () => {
    const { handle } = backend('running');
    const view = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    expect(view.jobs.filter(j => j.state === 'running')).toHaveLength(2);
    expect(await handle('lab_cancel_all', { project: 'frostveil' })).toBe(2);
    const after = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    expect(after.jobs.filter(j => j.state === 'cancelled')).toHaveLength(2);
  });

  it('holds the queue behind the Codex usage limit, every job waiting until it lifts', async () => {
    const { handle } = backend('usage');
    const view = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    const waiting = view.jobs.filter(j => j.state === 'rate-limited');
    expect(waiting).toHaveLength(3);
    expect(waiting.every(j => j.usageLimit && j.retryAtMs === waiting[0]!.retryAtMs)).toBe(true);
    expect(view.jobs.some(j => j.state === 'failed')).toBe(false);
  });

  it('runs a job that gave up on a rate limit again with Retry, under the same id', async () => {
    const { handle } = backend('limited');
    const view = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    const waiting = view.jobs.find(j => j.state === 'rate-limited');
    const failed = view.jobs.find(j => j.state === 'failed');
    expect(waiting?.retryAtMs).toBeGreaterThan(0);
    const again = (await handle('lab_retry', { jobId: failed!.id })) as { id: string };
    expect(again.id).toBe(failed!.id);
    await expect(handle('lab_retry', { jobId: view.jobs[0]!.id })).rejects.toThrow(/only a failed/);
    for (let i = 0; i < 200; i++) {
      const now = (await handle('lab_get', { project: 'frostveil' })) as LabView;
      if (now.jobs.find(j => j.id === failed!.id)?.state === 'done') break;
      await new Promise(r => setTimeout(r, 5));
    }
    const after = (await handle('lab_get', { project: 'frostveil' })) as LabView;
    expect(after.jobs.find(j => j.id === failed!.id)?.state).toBe('done');
    expect(after.jobs).toHaveLength(view.jobs.length);
    // Cancel all stops the one still waiting out the rate limit too.
    expect(await handle('lab_cancel_all', { project: 'frostveil' })).toBe(3);
  });
});

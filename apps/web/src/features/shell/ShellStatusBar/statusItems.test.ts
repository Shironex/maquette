import { describe, expect, it } from 'vitest';
import type { ConceptJob, ProviderJob, RunRecord, ToolReport } from '@/ipc/client';
import { shellStatusItems, type ShellStatus } from './statusItems';

const base: ShellStatus = {
  view: 'board',
  workspace: null,
  reports: [],
  checking: false,
  labJobs: [],
  inbox: null,
  latest: undefined,
  now: 100_000,
  mcp: null,
  sends: [],
};

const labels = (s: Partial<ShellStatus>) => shellStatusItems({ ...base, ...s }).map(i => i.label);

describe('the shell status items', () => {
  it('says Blender is missing and no run happened yet', () => {
    expect(labels({})).toEqual(['Local worker connected', 'Blender not found', 'No runs yet']);
  });

  it('names the Blender version, and the Codex CLI only in the Lab', () => {
    const reports = [
      { id: 'blender', status: 'found', version: '4.2' },
      { id: 'codex', status: 'missing' },
    ] as ToolReport[];
    expect(labels({ reports })).toContain('Blender 4.2, headless');
    expect(labels({ reports })).not.toContain('Codex CLI not ready');
    expect(labels({ reports, view: 'lab' })).toContain('Codex CLI not ready');
    expect(labels({ reports, checking: true })).toContain('Checking tools');
  });

  it('counts the images generating and waiting', () => {
    const job = (state: ConceptJob['state']) => ({ state }) as ConceptJob;
    expect(labels({ labJobs: [job('running'), job('queued'), job('queued')] })).toContain(
      '1 image generating, 2 waiting'
    );
    expect(labels({ labJobs: [job('queued')] })).toContain('1 image waiting');
  });

  it('counts the sends on the way and the ones waiting by hand', () => {
    const send = (state: ProviderJob['state']) => ({ state }) as ProviderJob;
    expect(labels({ sends: [send('running'), send('queued'), send('waiting')] })).toContain(
      '2 models on the way, 1 by hand'
    );
    expect(labels({ sends: [send('waiting')] })).toContain('1 waiting for your model');
  });

  it('says how long ago the last run finished', () => {
    const latest = { state: 'done', finishedAtMs: 40_000, queuedAtMs: 10_000 } as RunRecord;
    const items = shellStatusItems({ ...base, latest });
    expect(items.at(-1)).toEqual({ label: 'Last run 1 min ago', right: true });
  });
});

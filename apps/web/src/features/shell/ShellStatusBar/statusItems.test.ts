import { describe, expect, it } from 'vitest';
import type { ConceptJob, ProviderJob, RunRecord, ServerStatus, ToolReport } from '@/ipc/client';
import { mockUsage } from '@/ipc/mock/create-codex-usage-handlers';
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
  it('names the site dev page while it runs', () => {
    const server = (state: ServerStatus['state']) =>
      ({ state, url: 'http://localhost:4321/' }) as ServerStatus;
    expect(labels({ server: server('running') })).toContain('Site dev page on localhost:4321');
    expect(labels({ server: server('starting') })).toContain('Site dev page starting');
    expect(labels({ server: server('failed') })).toContain('Site dev page did not start');
    expect(labels({ server: server('idle') }).some(l => l.startsWith('Site dev page'))).toBe(false);
  });

  it('says where a running rig doctor case is', () => {
    expect(labels({ rigging: 'Rig doctor: pose test 9 of 13' })).toContain(
      'Rig doctor: pose test 9 of 13'
    );
    expect(labels({}).some(l => l.startsWith('Rig doctor'))).toBe(false);
  });

  it('hands the site dev page to its control, with its label and dot', () => {
    const seen: string[] = [];
    const items = shellStatusItems({
      ...base,
      server: { state: 'running', url: 'http://localhost:3000/' } as ServerStatus,
      siteControl: (label, dot) => {
        seen.push(`${label} ${dot}`);
        return label;
      },
    });
    expect(seen).toEqual(['Site dev page on localhost:3000 ok']);
    expect(items.find(i => i.label.startsWith('Site dev page'))?.node).toBe(
      'Site dev page on localhost:3000'
    );
  });

  it('names the library folder and its models in the Library view', () => {
    const workspace = {
      display: '~/Maquette',
      config: { layout: { inbox: 'inbox' } },
    } as ShellStatus['workspace'];
    expect(labels({ view: 'library', workspace, libraryModels: 8 })).toContain(
      'Library at ~/Maquette/library, 8 models'
    );
    expect(labels({ workspace, libraryModels: 8 }).some(l => l.startsWith('Library'))).toBe(false);
  });

  it('says Blender is missing and no run happened yet', () => {
    expect(labels({})).toEqual(['Local worker connected', 'Blender not found', 'No runs yet']);
  });

  it('says a Blender that fails did not run, not that it is gone', () => {
    const reports = [{ id: 'blender', status: 'broken' }] as ToolReport[];
    expect(labels({ reports })).toContain('Blender did not run');
    expect(labels({ reports })).not.toContain('Blender not found');
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

  it('shows the Codex usage in the Lab and while its images generate', () => {
    const codexUsage = mockUsage('low', 0);
    const item = (s: Partial<ShellStatus>) =>
      shellStatusItems({ ...base, codexUsage, ...s }).find(i => i.label.startsWith('Codex '));
    expect(item({ view: 'lab' })).toEqual({ label: 'Codex 86% (5h) · 41% (week)', dot: 'running' });
    expect(item({})).toBeUndefined();
    const running = { state: 'running' } as ConceptJob;
    expect(item({ labJobs: [running] })?.label).toBe('Codex 86% (5h) · 41% (week)');
    expect(item({ view: 'lab', codexUsage: mockUsage('limit', 0) })?.dot).toBe('offline');
    expect(item({ view: 'lab', codexUsage: mockUsage('ok', 0) })?.dot).toBe('ok');
    // A known read stands in for "Codex CLI, signed in".
    const reports = [{ id: 'codex', status: 'found' }] as ToolReport[];
    const lab = shellStatusItems({ ...base, view: 'lab', reports, codexUsage }).map(i => i.label);
    expect(lab).not.toContain('Codex CLI, signed in');
    expect(labels({ view: 'lab', reports })).toContain('Codex CLI, signed in');
    const drawn = item({ view: 'lab', codexControl: u => `control ${u.plan}` });
    expect(drawn?.node).toBe('control plus');
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

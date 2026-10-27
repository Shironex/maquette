import { describe, expect, it } from 'vitest';
import type { KeyStatus, ProviderJob } from '@/ipc/client';
import {
  cardState,
  creditsText,
  exportNameOf,
  formatElapsed,
  imagesNote,
  jobChip,
  jobMeta,
  jobTitle,
  latestJob,
  makeFront,
  sendBlocker,
  sortJobs,
  toggleImage,
} from './providers';

const job = (patch: Partial<ProviderJob>): ProviderJob => ({
  id: 'sakura-1',
  project: 'moon-island',
  asset: 'sakura',
  assetName: 'Sakura',
  provider: 'tripo',
  service: 'tripo',
  variant: 'prop',
  images: ['refs/sakura_front.png'],
  tripo: null,
  meshy: null,
  state: 'running',
  phase: 'generate',
  progress: 0.46,
  remoteId: null,
  credits: null,
  createdAtMs: 1_000,
  startedAtMs: 2_000,
  finishedAtMs: null,
  message: null,
  inboxFile: null,
  handoff: null,
  card: null,
  version: null,
  viaAgent: false,
  cancel: 'none',
  batch: null,
  ...patch,
});

const keys: KeyStatus[] = [
  { service: 'tripo', saved: true, tail: '9f2c' },
  { service: 'meshy', saved: false, tail: null },
];

describe('provider jobs', () => {
  it('words every state for the chip and the card', () => {
    expect(jobChip(job({}))).toEqual({ status: 'running', label: '46%' });
    expect(jobChip(job({ state: 'waiting' })).label).toBe('Waiting for your model');
    expect(jobChip(job({ state: 'approval' })).label).toBe('Waiting for approval');
    expect(jobChip(job({ state: 'accepted', version: 3 })).label).toBe('Accepted as v3');
    expect(cardState(job({}))).toMatchObject({ progress: 46, progressLabel: 'Tripo, generating' });
    expect(cardState(job({ state: 'failed', provider: 'meshy' }))?.label).toBe('Meshy failed');
    expect(cardState(job({ state: 'queued' }))?.label).toBe('Queued for Tripo');
    expect(cardState(job({ state: 'accepted' }))).toBeNull();
    expect(cardState(undefined)).toBeNull();
  });

  it('titles, credits and time', () => {
    expect(jobTitle(job({ provider: 'by-hand', service: 'meshy' }))).toBe(
      'By hand, Meshy settings'
    );
    expect(jobTitle(job({ state: 'queued' }))).toBe('Tripo');
    expect(creditsText(job({ credits: 30 }))).toBe('30 credits');
    expect(creditsText(job({ credits: 1 }))).toBe('1 credit');
    expect(creditsText(job({ provider: 'by-hand' }))).toBe('no credits');
    expect(creditsText(job({}))).toBeNull();
    expect(formatElapsed(42_000)).toBe('42s');
    expect(formatElapsed(72_000)).toBe('1m 12s');
    expect(formatElapsed(2 * 3_600_000 + 5 * 60_000)).toBe('2h 5m');
    expect(jobMeta(job({ credits: 30, viaAgent: true }), 74_000)).toBe(
      '1 image · 1m 12s · 30 credits · asked by an agent'
    );
  });

  it('puts active jobs first and finds the newest per asset', () => {
    const done = job({ id: 'sakura-1', state: 'inbox', createdAtMs: 5 });
    const running = job({ id: 'sakura-2', createdAtMs: 1 });
    expect(sortJobs([done, running]).map(j => j.id)).toEqual(['sakura-2', 'sakura-1']);
    expect(latestJob([done, running], 'sakura')?.id).toBe('sakura-1');
    expect(latestJob([done], 'lanterns')).toBeUndefined();
  });
});

describe('the send panel', () => {
  it('blocks a send without images or a key', () => {
    expect(sendBlocker('tripo', keys, [], null)).toBe('Pick at least one image.');
    expect(sendBlocker('meshy', keys, ['a'], null)).toContain('Meshy API key');
    expect(sendBlocker('by-hand', keys, ['a'], null)).toBeNull();
    expect(sendBlocker('tripo', keys, ['a'], 'Too many faces.')).toBe('Too many faces.');
    expect(sendBlocker('tripo', keys, ['a'], null)).toBeNull();
  });

  it('says what the images become', () => {
    expect(imagesNote('tripo', 'tripo', ['a', 'b'], true, false)).toContain('multi-view');
    expect(imagesNote('tripo', 'tripo', ['a', 'b'], false, false)).toContain('only the first');
    expect(imagesNote('meshy', 'meshy', ['a', 'b'], true, true)).toContain('Smart Topology');
    expect(imagesNote('by-hand', 'tripo', ['a', 'b'], true, false)).toContain('named by view');
  });

  it('keeps at most four images, front first', () => {
    expect(toggleImage(['a', 'b', 'c', 'd'], 'e')).toEqual(['a', 'b', 'c', 'd']);
    expect(toggleImage(['a', 'b'], 'a')).toEqual(['b']);
    expect(toggleImage(['a'], 'b')).toEqual(['a', 'b']);
    expect(makeFront(['a', 'b', 'c'], 'c')).toEqual(['c', 'a', 'b']);
  });
});

describe('the export name of a By hand card', () => {
  it('reads the card, or the file name of an older card', () => {
    expect(exportNameOf({ exportName: 'maq_lamp_v1_tripo', returnName: 'x.glb' })).toBe(
      'maq_lamp_v1_tripo'
    );
    expect(exportNameOf({ exportName: '', returnName: 'maq_lamp_v1_tripo.glb' })).toBe(
      'maq_lamp_v1_tripo'
    );
  });
});

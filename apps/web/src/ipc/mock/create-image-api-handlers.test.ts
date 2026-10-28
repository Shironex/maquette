import { describe, expect, it } from 'vitest';
import type { ImageApiStatus, LabView, OpenAiKeyTest } from '../client';
import { createBackend } from './backend';

const request = (count: number) => ({
  request: {
    project: 'frostveil',
    direction: 'Harbour',
    prompt: 'A small harbour at dawn',
    count,
    refs: [],
    aspect: '3:2' as const,
    useStyle: false,
  },
});

const backend = (images: string | null) =>
  createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, conceptMs: 60_000, images });

describe('the mock image api', () => {
  it('keeps every kind on codex with no key by default', async () => {
    const handle = backend(null);
    const status = (await handle('image_api_status')) as ImageApiStatus;
    expect(status.key).toEqual({ saved: false, tail: null });
    expect(status.settings.backends?.concepts).toBe('codex');
    const view = (await handle('lab_generate', request(2))) as LabView;
    expect(view.jobs.some(j => j.backend === 'openai-images')).toBe(false);
  });

  it('marks concepts made on openai images', async () => {
    const handle = backend('on');
    const status = (await handle('image_api_status')) as ImageApiStatus;
    expect(status.monthSpentUsd).toBe(3.42);
    const view = (await handle('lab_generate', request(2))) as LabView;
    const harbour = view.jobs.filter(j => j.direction === 'harbour');
    expect(harbour.map(j => j.backend)).toEqual(['openai-images', 'openai-images']);
    const after = (await handle('image_api_status')) as ImageApiStatus;
    expect(after.pendingUsd).toBeCloseTo(0.022);
  });

  it('refuses a round that would pass the cap', async () => {
    const handle = backend('cap');
    await expect(handle('lab_generate', request(4))).rejects.toMatchObject({
      message: expect.stringMatching(/monthly OpenAI image cap/),
    });
  });

  it('tests a key by listing the models', async () => {
    const handle = backend('on');
    const good = (await handle('image_api_key_test', { key: null })) as OpenAiKeyTest;
    expect(good).toMatchObject({ ok: true, hasModel: true });
    const bad = (await handle('image_api_key_test', { key: 'sk-bad-key' })) as OpenAiKeyTest;
    expect(bad.ok).toBe(false);
  });
});

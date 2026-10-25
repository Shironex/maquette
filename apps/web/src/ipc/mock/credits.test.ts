import { describe, expect, it } from 'vitest';
import type { Balance, Ledger, ProviderJob, SendRequest } from '../client';
import { createBackend } from './backend';
import { defaultConfig } from './fixtures';
import { meshyCredits, tripoCredits } from './pricing';

const presets = defaultConfig().providers;

describe('the mock price lists', () => {
  it('match the core for Tripo', () => {
    const prop = presets.tripo.prop;
    expect(tripoCredits(prop, 'prop')).toBe(30);
    expect(tripoCredits({ ...prop, texture: false }, 'prop')).toBe(20);
    expect(
      tripoCredits({ ...prop, textureQuality: 'detailed', geometryQuality: 'detailed' }, 'prop')
    ).toBe(60);
    // Rig 25 and 10 per animation, only for a character.
    expect(tripoCredits(presets.tripo.character, 'character')).toBe(75);
    expect(tripoCredits(presets.tripo.character, 'prop')).toBe(30);
  });

  it('match the core for Meshy', () => {
    const prop = presets.meshy.prop;
    expect(meshyCredits(prop, 'prop')).toBe(30);
    expect(meshyCredits({ ...prop, textureResolution: '8k' }, 'prop')).toBe(35);
    expect(meshyCredits({ ...prop, shouldTexture: false }, 'prop')).toBe(20);
    expect(meshyCredits({ ...prop, aiModel: 'meshy-t2' }, 'prop')).toBe(15);
  });
});

describe('mock credits', () => {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, inbox: 'redo' });

  it('estimates a send from its settings, or the saved preset, and nothing by hand', async () => {
    const request: SendRequest = {
      project: 'moon-island',
      asset: 'sakura',
      provider: 'tripo',
      service: 'tripo',
      variant: 'prop',
      images: ['refs/sakura_front.png'],
      tripo: null,
      meshy: null,
    };
    expect(await handle('provider_estimate', { request })).toBe(30);
    expect(
      await handle('provider_estimate', { request: { ...request, provider: 'by-hand' } })
    ).toBeNull();
  });

  it('lists the live paid sends with a month of earlier ones, newest first', async () => {
    const ledger = (await handle('credits_ledger', {})) as Ledger;
    const jobs = ledger.spends.map(s => s.job);
    // The running Tripo send holds its estimate; the failed Meshy one is free.
    expect(ledger.spends.find(s => s.job === 'sakura-1')).toMatchObject({
      outcome: 'pending',
      estimated: true,
      credits: 30,
    });
    expect(ledger.spends.find(s => s.job === 'far-spire-1')).toMatchObject({ credits: 0 });
    expect(jobs).toContain('character-3');
    expect(jobs).not.toContain('lanterns-1');
    const times = ledger.spends.map(s => s.sentAtMs ?? 0);
    expect([...times].sort((a, b) => b - a)).toEqual(times);
  });

  it('asks for a balance only with a saved key', async () => {
    const balances = (await handle('credits_balances', {})) as Balance[];
    expect(balances.map(b => [b.service, b.credits])).toEqual([
      ['tripo', 1240],
      ['meshy', 380],
    ]);
    const plain = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0 });
    expect(((await plain('credits_balances', {})) as Balance[])[1]).toMatchObject({
      key: false,
      credits: null,
    });
  });

  it('links the redo fixture inbox models to their sends', async () => {
    const jobs = (await handle('provider_jobs', { project: 'moon-island' })) as ProviderJob[];
    expect(jobs.find(j => j.id === 'sakura-2')).toMatchObject({ provider: 'meshy', estimate: 30 });
    expect(jobs.find(j => j.id === 'character-3')).toMatchObject({ variant: 'character' });
  });
});

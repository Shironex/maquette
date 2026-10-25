import { describe, expect, it } from 'vitest';
import type { InboxItem, ProviderJob, SendOptions } from '@/ipc/client';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { batchDraft, batchRequest } from './batch';
import {
  batchRedoPlan,
  redoBatchDraft,
  redoBlocker,
  redoDraft,
  redoLine,
  redoUnderWay,
} from './redo';
import type { SendDraft } from './providers';

const presets = defaultConfig().providers;

const job = (patch: Partial<ProviderJob>): ProviderJob => ({
  id: 'sakura-2',
  project: 'moon-island',
  asset: 'sakura',
  assetName: 'Sakura',
  provider: 'meshy',
  service: 'meshy',
  variant: 'prop',
  images: ['refs/sakura_front.png', 'refs/gone.png'],
  tripo: null,
  meshy: { ...presets.meshy.prop, textureResolution: '4k' },
  state: 'inbox',
  phase: null,
  progress: 1,
  remoteId: null,
  credits: 30,
  createdAtMs: 1,
  startedAtMs: 2,
  finishedAtMs: 3,
  message: null,
  inboxFile: 'sakura_large.glb',
  handoff: null,
  card: null,
  version: null,
  viaAgent: false,
  cancel: 'none',
  batch: null,
  prompt: 'Pale pink blossoms',
  ...patch,
});

const options = {
  images: [{ file: 'refs/sakura_front.png', name: 'sakura_front.png', bytes: 1 }],
  suggested: ['refs/sakura_front.png'],
} as unknown as SendOptions;

const fresh: SendDraft = {
  provider: 'tripo',
  service: 'tripo',
  variant: 'prop',
  images: ['refs/fresh.png'],
  tripo: structuredClone(presets.tripo.prop),
  meshy: structuredClone(presets.meshy.prop),
};

const link = (job: string, asset: string, project = 'moon-island') => ({
  job,
  project,
  asset,
  assetName: asset,
  provider: 'tripo' as const,
});

const item = (id: string, l: InboxItem['link']) => ({ id, name: id, link: l }) as InboxItem;

describe('a redo of one send', () => {
  it('repeats the provider, kind, images, settings and prompt', () => {
    const draft = redoDraft(job({}), options, fresh);
    expect(draft).toMatchObject({ provider: 'meshy', service: 'meshy', variant: 'prop' });
    // An image gone from refs/ is dropped.
    expect(draft.images).toEqual(['refs/sakura_front.png']);
    expect(draft.meshy.textureResolution).toBe('4k');
    expect(draft.tripo).toEqual(fresh.tripo);
    expect(draft.prompt).toBe('Pale pink blossoms');
  });

  it('picks the asset refs again when none of its images are left', () => {
    const draft = redoDraft(job({ images: ['refs/gone.png'], meshy: null }), options, fresh);
    expect(draft.images).toEqual(['refs/fresh.png']);
    expect(draft.meshy).toEqual(fresh.meshy);
  });

  it('needs a send to repeat', () => {
    expect(redoBlocker({ link: null })).toMatch(/did not come from a Send to 3D/);
    expect(redoBlocker({ link: link('sakura-2', 'sakura') })).toBeNull();
    expect(redoLine(job({}))).toBe('Redo of sakura-2, sent to Meshy');
  });

  it('finds a redo still on its way', () => {
    const redo = job({ id: 'sakura-3', state: 'running', redoOf: 'sakura-2' });
    const done = job({ id: 'sakura-4', state: 'inbox', redoOf: 'sakura-2' });
    const linked = { link: link('sakura-2', 'sakura') };
    expect(redoUnderWay([done, redo], linked)?.id).toBe('sakura-3');
    expect(redoUnderWay([done], linked)).toBeUndefined();
    expect(redoUnderWay([redo], { link: null })).toBeUndefined();
  });
});

describe('a batch redo', () => {
  const base = batchDraft(
    {
      project: 'moon-island',
      projectDir: '/w',
      keys: [],
      items: [
        {
          asset: { id: 'character', name: 'Character', thumbnail: null },
          suggested: [],
          variant: 'prop',
        },
        {
          asset: { id: 'sakura', name: 'Sakura', thumbnail: null },
          suggested: [],
          variant: 'prop',
        },
        { asset: { id: 'lamp', name: 'Lamp', thumbnail: null }, suggested: [], variant: 'prop' },
      ],
    } as never,
    presets.tripo,
    presets.meshy
  );
  const character = job({
    id: 'character-3',
    asset: 'character',
    provider: 'tripo',
    service: 'tripo',
    variant: 'character',
    images: ['refs/character-model.png'],
    tripo: { ...presets.tripo.character, textureQuality: 'detailed' },
    meshy: null,
    prompt: null,
  });
  const lamp = job({
    id: 'lamp-1',
    asset: 'lamp',
    provider: 'tripo',
    service: 'tripo',
    images: ['refs/lamp.png'],
    tripo: { ...presets.tripo.prop, faceLimit: 12_000 },
    meshy: null,
    prompt: null,
  });
  const sakura = job({});

  it('goes to the provider most of them went to, each asset from its own send', () => {
    const draft = redoBatchDraft(base, { character, sakura, lamp });
    expect(draft).toMatchObject({ provider: 'tripo', service: 'tripo' });
    // Each kind's batch preset is the first send of that kind.
    expect(draft.tripo.character.textureQuality).toBe('detailed');
    expect(draft.tripo.prop.faceLimit).toBe(12_000);
    const [c, s, l] = draft.items;
    expect(c).toMatchObject({ variant: 'character', redoOf: 'character-3', tripo: {} });
    expect(c!.images).toEqual(['refs/character-model.png']);
    // The Meshy send keeps its images and prompt, and follows the Tripo batch preset.
    expect(s).toMatchObject({ redoOf: 'sakura-2', prompt: 'Pale pink blossoms', tripo: {} });
    expect(l).toMatchObject({ redoOf: 'lamp-1', tripo: {} });
    const request = batchRequest('moon-island', draft);
    expect(request.items.map(i => i.redoOf)).toEqual(['character-3', 'sakura-2', 'lamp-1']);
    expect(request.items.every(i => i.prompt === null)).toBe(true);
  });

  it('keeps what an asset changed from its kind preset as its own', () => {
    const other = job({
      ...lamp,
      id: 'lamp-2',
      asset: 'sakura',
      tripo: { ...presets.tripo.prop, pbr: false },
    });
    const draft = redoBatchDraft(base, { lamp, sakura: other });
    // The sakura comes first in the list, so its settings are the props' batch preset.
    expect(draft.tripo.prop.pbr).toBe(false);
    expect(draft.items.find(i => i.asset === 'sakura')!.tripo).toEqual({});
    expect(draft.items.find(i => i.asset === 'lamp')!.tripo).toEqual({
      faceLimit: 12_000,
      pbr: true,
    });
  });

  it('plans one batch of linked models from one project', () => {
    const items = [
      item('a.glb', link('sakura-2', 'sakura')),
      item('b.glb', link('character-3', 'character')),
      item('c.glb', null),
      item('d.glb', link('vending-1', 'vending', 'rooftop-cinema')),
      item('e.glb', link('sakura-1', 'sakura')),
    ];
    expect(batchRedoPlan(items, ['a.glb', 'b.glb'])).toEqual({
      project: 'moon-island',
      redo: [
        { asset: 'sakura', job: 'sakura-2' },
        { asset: 'character', job: 'character-3' },
      ],
      blocker: null,
    });
    expect(batchRedoPlan(items, []).blocker).toBe('Select the models to redo');
    expect(batchRedoPlan(items, ['a.glb', 'c.glb']).blocker).toBe(
      'c.glb did not come from a Send to 3D'
    );
    expect(batchRedoPlan(items, ['a.glb', 'd.glb']).blocker).toMatch(/different projects/);
    expect(batchRedoPlan(items, ['a.glb', 'e.glb']).blocker).toMatch(/one per asset/);
  });
});

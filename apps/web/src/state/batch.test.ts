import { describe, expect, it } from 'vitest';
import type { BatchOptions, DownloadsScan, ProviderJob } from '@/ipc/client';
import { defaultConfig } from '@/ipc/mock/fixtures';
import {
  backLabel,
  batchBlocker,
  batchDraft,
  batchLabel,
  batchLines,
  batchProblem,
  batchRequest,
  batchSummary,
  checklist,
  initialMatches,
  isCustom,
  itemDiff,
  itemTripo,
  pruneItems,
  nextSelection,
  presetChanges,
  waitingByHand,
} from './batch';

const order = ['a', 'b', 'c', 'd', 'e'];

describe('the Board selection', () => {
  it('adds and drops single cards in Board order', () => {
    expect(nextSelection([], order, 'c', true, false, null)).toEqual(['c']);
    expect(nextSelection(['c'], order, 'a', true, false, 'c')).toEqual(['a', 'c']);
    expect(nextSelection(['a', 'c'], order, 'a', false, false, 'c')).toEqual(['c']);
  });

  it('takes every card between the last click and a shift click', () => {
    expect(nextSelection(['b'], order, 'd', true, true, 'b')).toEqual(['b', 'c', 'd']);
    expect(nextSelection(['b', 'c', 'd', 'e'], order, 'c', false, true, 'e')).toEqual(['b']);
    // Without an anchor a shift click is a plain click.
    expect(nextSelection([], order, 'd', true, true, null)).toEqual(['d']);
  });
});

const presets = defaultConfig().providers;

function options(): BatchOptions {
  const asset = (id: string, name: string, kind: 'prop' | 'character') => ({
    id,
    name,
    kind,
    stage: 'refs' as const,
    status: 'queued' as const,
    versions: [],
    thumbnail: `refs/${id}.png`,
    createdAt: '2026-10-24T10:00:00Z',
    updatedAt: '2026-10-24T10:00:00Z',
  });
  return {
    project: 'moon-island',
    projectDir: '/w/projects/moon-island',
    items: [
      {
        asset: asset('desk-lamp', 'Desk lamp', 'prop'),
        suggested: ['refs/desk_lamp.png'],
        variant: 'prop',
      },
      {
        asset: asset('sculptor', 'Sculptor figure', 'character'),
        suggested: ['refs/sculptor.png'],
        variant: 'character',
      },
    ],
    keys: [],
  };
}

describe('a batch send', () => {
  it('starts By hand with the saved presets and each asset its own kind', () => {
    const draft = batchDraft(options(), presets.tripo, presets.meshy);
    expect(draft.provider).toBe('by-hand');
    expect(draft.items.map(i => i.variant)).toEqual(['prop', 'character']);
    expect(batchSummary(draft)).toBe('2 models · 1 prop, 1 character · By hand, Tripo settings');
    expect(batchLabel(draft)).toBe('Write 2 handoff folders');
  });

  it('sends only the presets the items use, for the chosen service', () => {
    const draft = batchDraft(options(), presets.tripo, presets.meshy, 'meshy');
    const props = { ...draft, items: draft.items.map(i => ({ ...i, variant: 'prop' as const })) };
    const request = batchRequest('moon-island', props);
    expect(request.service).toBe('meshy');
    expect(request.meshyProp).toEqual(presets.meshy.prop);
    expect(request.meshyCharacter).toBeNull();
    expect(request.tripoProp).toBeNull();
    expect(request.items).toEqual([
      {
        asset: 'desk-lamp',
        variant: 'prop',
        images: ['refs/desk_lamp.png'],
        tripo: null,
        meshy: null,
      },
      {
        asset: 'sculptor',
        variant: 'prop',
        images: ['refs/sculptor.png'],
        tripo: null,
        meshy: null,
      },
    ]);
    expect(batchLabel(props)).toBe('Send 2 to Meshy');
  });

  it('says what blocks it', () => {
    const draft = batchDraft(options(), presets.tripo, presets.meshy, 'tripo');
    expect(batchBlocker(draft, [], null)).toBe(
      'Add a Tripo API key in Settings > 3D providers first.'
    );
    const keys = [{ service: 'tripo' as const, saved: true, tail: '9f2c' }];
    expect(batchBlocker(draft, keys, null)).toBeNull();
    const bare = { ...draft, items: [{ ...draft.items[0]!, images: [] }] };
    expect(batchBlocker(bare, keys, null)).toBe(
      'Desk lamp has no refs yet. Send a pick to Refs first.'
    );
    expect(batchBlocker({ ...draft, items: [] }, keys, null)).toBe('Select at least one asset.');
    expect(batchBlocker(draft, keys, 'Pick a model.')).toBe('Pick a model.');
  });
});

describe('per-asset overrides', () => {
  const catalog = {
    tripoModels: [{ id: 'v3.1-20260211', label: 'H3.1' }],
    meshyModels: [],
  } as unknown as Parameters<typeof batchLines>[1];
  const draft = () => {
    const d = batchDraft(options(), presets.tripo, presets.meshy, 'tripo');
    return { ...d, items: d.items.map(i => ({ ...i, variant: 'prop' as const })) };
  };

  it('keeps only the changed fields', () => {
    const base = presets.tripo.prop;
    expect(presetChanges(base, base)).toEqual({});
    expect(presetChanges(base, { ...base, faceLimit: 12_000, pbr: !base.pbr })).toEqual({
      faceLimit: 12_000,
      pbr: !base.pbr,
    });
    expect(presetChanges(base, { ...base, animations: [...base.animations] })).toEqual({});
  });

  it('lets batch edits flow into the fields an asset did not change', () => {
    const d = draft();
    d.items[0] = { ...d.items[0]!, tripo: { faceLimit: 12_000 } };
    d.tripo = { ...d.tripo, prop: { ...d.tripo.prop, faceLimit: 5_000, pbr: false } };
    expect(isCustom(d, d.items[0])).toBe(true);
    expect(isCustom(d, d.items[1]!)).toBe(false);
    expect(itemTripo(d, d.items[0])).toEqual({ ...d.tripo.prop, faceLimit: 12_000 });
    expect(itemTripo(d, d.items[1]!).faceLimit).toBe(5_000);
  });

  it('drops an override once the batch moves to the same value', () => {
    const d = draft();
    const base = d.tripo.prop;
    d.items[0] = { ...d.items[0]!, tripo: { autoSize: !base.autoSize, faceLimit: 2_000 } };
    d.tripo = { ...d.tripo, prop: { ...base, autoSize: !base.autoSize } };
    expect(isCustom(d, d.items[0])).toBe(true);
    expect(itemDiff(d, d.items[0], catalog)).toBe('2,000 faces');
    expect(batchLines(d, catalog).at(-1)).toBe(`${d.items[0].name}: 2,000 faces`);
    const only = { ...d, items: [{ ...d.items[0], tripo: { autoSize: !base.autoSize } }] };
    expect(isCustom(only, only.items[0]!)).toBe(false);
    expect(batchRequest('moon-island', only).items[0]?.tripo).toBeNull();
    expect(pruneItems(d).items[0]?.tripo).toEqual({ faceLimit: 2_000 });
  });

  it('sends the merged preset for custom assets only', () => {
    const d = draft();
    d.items[1] = { ...d.items[1]!, tripo: { faceLimit: 2_000 } };
    const request = batchRequest('moon-island', d);
    expect(request.items[0]?.tripo).toBeNull();
    expect(request.items[1]?.tripo).toEqual({ ...presets.tripo.prop, faceLimit: 2_000 });
    expect(request.items[1]?.meshy).toBeNull();
  });

  it('lists what each custom asset changes next to the batch settings', () => {
    const d = draft();
    d.items[1] = { ...d.items[1]!, tripo: { faceLimit: 2_000, pbr: false } };
    expect(batchLines(d, catalog)).toEqual([
      expect.stringMatching(/^Props: Tripo H3\.1/),
      'Sculptor figure: 2,000 faces, no PBR',
    ]);
  });

  it('names the asset whose own settings are wrong', () => {
    const d = draft();
    expect(batchProblem(d, catalog)).toBeNull();
    d.items[1] = { ...d.items[1]!, tripo: { faceLimit: 99_000_000 } };
    expect(batchProblem(d, catalog)).toMatch(/^Sculptor figure: Tripo/);
  });
});

const job = (patch: Partial<ProviderJob>): ProviderJob => ({
  id: 'x-1',
  project: 'moon-island',
  asset: 'x',
  assetName: 'X',
  provider: 'by-hand',
  service: 'tripo',
  variant: 'prop',
  images: [],
  tripo: null,
  meshy: null,
  state: 'waiting',
  phase: null,
  progress: null,
  remoteId: null,
  credits: null,
  createdAtMs: 1_000,
  startedAtMs: null,
  finishedAtMs: null,
  message: null,
  inboxFile: null,
  handoff: null,
  card: null,
  version: null,
  viaAgent: false,
  cancel: 'stop-waiting',
  batch: null,
  ...patch,
});

describe('the By hand checklist', () => {
  it('groups sends by batch, newest first, and counts what came back', () => {
    const jobs = [
      job({ id: 'lamp-1', assetName: 'Desk lamp', batch: 'b1', createdAtMs: 5_000 }),
      job({ id: 'cup-1', assetName: 'Brush cup', batch: 'b1', state: 'inbox', createdAtMs: 5_000 }),
      job({ id: 'old-1', assetName: 'Old', createdAtMs: 1_000 }),
      job({ id: 'gone-1', assetName: 'Gone', state: 'cancelled', createdAtMs: 9_000 }),
      job({ id: 'api-1', provider: 'tripo', createdAtMs: 9_000 }),
    ];
    const groups = checklist(jobs);
    expect(groups.map(g => g.batch)).toEqual(['b1', null]);
    expect(groups[0]!.jobs.map(j => j.assetName)).toEqual(['Brush cup', 'Desk lamp']);
    expect(backLabel(groups[0]!)).toBe('1 of 2 back');
    expect(backLabel({ jobs: [job({ state: 'accepted' })], back: 1 })).toBe('All 1 back');
    expect(waitingByHand(jobs).map(j => j.id)).toEqual(['lamp-1', 'old-1']);
  });

  it('starts every Downloads file at its own match', () => {
    const scan: DownloadsScan = {
      folder: '/Users/maker/Downloads',
      sinceMs: 1_000,
      waiting: [],
      files: [
        {
          path: '/d/desk_lamp.glb',
          name: 'desk_lamp.glb',
          inner: null,
          bytes: 1,
          modifiedMs: 2_000,
          job: 'lamp-1',
          assetName: 'Desk lamp',
          reason: 'Named like Desk lamp',
        },
        {
          path: '/d/model.glb',
          name: 'model.glb',
          inner: null,
          bytes: 1,
          modifiedMs: 2_000,
          job: null,
          assetName: null,
          reason: null,
        },
      ],
    };
    expect(initialMatches(scan)).toEqual({ '/d/desk_lamp.glb': 'lamp-1', '/d/model.glb': null });
  });
});

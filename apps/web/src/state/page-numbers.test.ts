import { describe, expect, it } from 'vitest';
import type { InspectModel, ModelSource } from '@/ipc/client';
import { DEVICE_BUDGETS, compareRows, modelChanges, sceneNumbers, verdict } from './page-numbers';

function model(
  source: ModelSource,
  stem: string,
  bytes: number,
  patch: Partial<InspectModel> = {}
): InspectModel {
  return {
    id: `${source}/${stem}`,
    model: stem,
    label: stem.replace(/^./, c => c.toUpperCase()),
    source,
    bytes,
    triangles: 10_000,
    materials: 1,
    gpuBytes: 20e6,
    error: null,
    ...patch,
  } as InspectModel;
}

const MODELS = [
  model('site', 'sakura', 420_000, { triangles: 30_000 }),
  model('site', 'lantern', 90_000),
  model('shipped', 'sakura', 330_000, { triangles: 28_000 }),
  model('shipped', 'character', 1_500_000, { materials: 2, gpuBytes: 60e6 }),
  model('raw', 'sakura', 4_000_000),
];

describe('scene numbers', () => {
  it('totals one source', () => {
    expect(sceneNumbers(MODELS, 'shipped')).toEqual({
      models: 2,
      bytes: 1_830_000,
      triangles: 38_000,
      gpuBytes: 80e6,
      drawCalls: 3,
    });
    expect(sceneNumbers(MODELS, 'version')).toBeNull();
  });

  it('compares the ship with this run, lower is better', () => {
    const rows = compareRows(sceneNumbers(MODELS, 'site'), sceneNumbers(MODELS, 'shipped'));
    expect(rows.map(r => [r.label, r.ship, r.run, r.change, r.better])).toEqual([
      ['Scene size', '0.5 MB', '1.8 MB', '+1.3 MB', false],
      ['Triangles', '40,000', '38,000', '-2,000', true],
      ['GPU memory', '40 MB', '80 MB', '+40 MB', false],
      ['Draw calls', '2', '3', '+1', false],
    ]);
  });

  it('says none for a side without models', () => {
    const rows = compareRows(null, sceneNumbers(MODELS, 'shipped'));
    expect(rows[0]).toEqual({
      label: 'Scene size',
      ship: 'none',
      run: '1.8 MB',
      change: '',
      better: null,
    });
  });

  it('lists what changed per model, the biggest first', () => {
    expect(modelChanges(MODELS)).toEqual([
      { title: 'Character is new', detail: 'Adds 1.5 MB to the scene' },
      { title: 'Sakura', detail: '90 KB lighter, 2,000 triangles fewer' },
      { title: 'Lantern leaves', detail: 'Saves 90 KB' },
    ]);
  });
});

describe('the device verdict', () => {
  it('flags what is over the phone budget', () => {
    const rows = verdict(DEVICE_BUDGETS.phone, sceneNumbers(MODELS, 'shipped'), 1_800_000, 38);
    expect(rows.map(r => [r.label, r.value, r.budget, r.over])).toEqual([
      ['Frame time, p95', '38.0 ms', '33 ms', true],
      ['First view', '1.8 MB', '1.5 MB', true],
      ['GPU memory', '80 MB', '256 MB', false],
      ['Draw calls', '3', '120', false],
    ]);
    expect(verdict(DEVICE_BUDGETS.desktop, null, null, null)).toEqual([]);
  });
});

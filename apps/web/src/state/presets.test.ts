import { describe, expect, it } from 'vitest';
import { CHARACTER_LOCK, pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { estimatedTransfer } from './pipeline';
import { estimateTransfer, modelLabel, presetRows, presetsToSave, splitSize } from './presets';

const view = pipelineView('frostveil', '/w', runHistory(0), []);

describe('presets', () => {
  it('labels an export of an asset version after the asset', () => {
    const clay = {
      ...view,
      models: [
        { model: 'clay_v1', assetName: 'clay', version: 1, rawBytes: 1, shippedBytes: null },
      ],
      def: { ...view.def, presets: [] },
    };
    expect(presetRows(clay).map(r => [r.model, r.label])).toEqual([['clay_v1', 'clay']]);
  });

  it('labels model stems', () => {
    expect(modelLabel('veil_gate')).toBe('Veil gate');
    expect(modelLabel('observatory-arch')).toBe('Observatory arch');
  });

  it('lists every model with stored rows or the defaults', () => {
    const rows = presetRows(view);
    expect(rows.map(r => r.model)).toEqual([
      'crystal_lantern',
      'elsheria',
      'observatory_arch',
      'veil_gate',
      'veil_platform',
    ]);
    const elsheria = rows.find(r => r.model === 'elsheria')!;
    expect(elsheria).toMatchObject({ meshopt: 'medium', lock: CHARACTER_LOCK, custom: true });
    expect(rows.find(r => r.model === 'veil_platform')).toMatchObject({
      textureSize: 2048,
      codec: 'etc1s',
      meshopt: 'high',
      custom: false,
    });
  });

  it('saves stored and edited rows only, keeping locks', () => {
    const rows = presetRows(view).map(r =>
      r.model === 'veil_platform' ? { ...r, textureSize: 1024, custom: true } : r
    );
    const saved = presetsToSave(rows);
    expect(saved.map(p => p.model)).toEqual([
      'crystal_lantern',
      'elsheria',
      'veil_gate',
      'veil_platform',
    ]);
    expect(saved.find(p => p.model === 'veil_platform')!.textureSize).toBe(1024);
    expect(saved.find(p => p.model === 'elsheria')!.lock).toBe(CHARACTER_LOCK);
    expect(Object.keys(saved[0]!)).not.toContain('custom');
  });

  it('adds up shipped bytes', () => {
    expect(estimatedTransfer(view)).toBeCloseTo(3.47e6);
  });

  it('keeps one size by default and saves a split only when set', () => {
    const rows = presetRows(view);
    expect(rows.every(r => r.dataTextureSize === null)).toBe(true);
    const saved = presetsToSave(
      rows.map(r =>
        r.model === 'veil_platform' ? { ...r, dataTextureSize: 1024, custom: true } : r
      )
    );
    expect(saved.find(p => p.model === 'veil_platform')!.dataTextureSize).toBe(1024);
    expect(saved.find(p => p.model === 'veil_gate')!.dataTextureSize).toBeNull();
    expect(splitSize({ textureSize: 2048, dataTextureSize: 1024 })).toBe(1024);
    expect(splitSize({ textureSize: 1024, dataTextureSize: 2048 })).toBeNull();
    expect(splitSize({ textureSize: 1024, dataTextureSize: null })).toBeNull();
  });

  it('estimates a smaller transfer when data maps shrink, and none when nothing changed', () => {
    const rows = presetRows(view);
    expect(estimateTransfer(view, rows)).toBeCloseTo(estimatedTransfer(view));
    const split = rows.map(r =>
      r.model === 'veil_platform' ? { ...r, dataTextureSize: 512, custom: true } : r
    );
    const island = view.models.find(m => m.model === 'veil_platform')!;
    const base = island.shippedBytes ?? island.rawBytes ?? 0;
    const estimate = estimateTransfer(view, split);
    expect(estimate).toBeLessThan(estimatedTransfer(view));
    expect(estimatedTransfer(view) - estimate).toBeLessThan(base);
    // Undoing the split brings it back.
    expect(estimateTransfer(view, rows)).toBeGreaterThan(estimate);
  });
});
